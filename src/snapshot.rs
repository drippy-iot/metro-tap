use core::time::Duration;
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Input, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Flow, MacAddress};

use crate::{
    flow::take_ticks,
    http::{report_flow, report_leak, HttpClient},
    SharedOutputPin,
};

pub async fn report<Tap: Pin, Valve: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    mut http: HttpClient,
    tap: PinDriver<'_, Tap, Input>,
    valve: SharedOutputPin<'_, Valve>,
) -> Result<(), EspError> {
    const SECONDS: u16 = 3;
    loop {
        timer.after(Duration::from_secs(SECONDS.into()))?.await;
        let flow = take_ticks();
        let unit = flow / SECONDS;
        log::info!("{flow} total ticks (i.e., {unit} ticks per second) detected since last reset");

        // TODO: In the future, we may want to piggy-back the leak detection
        // to the regular reporting instead. Not only is it more network-efficient,
        // but we also allow the Cloud to handle all leak-related logic.

        // Check if water is passing through while the tap is closed
        if tap.is_low() && flow > 10 {
            if report_leak(&mut http, &addr.0).await.map_err(|EspIOError(err)| err)? {
                valve.lock().unwrap().set_low()?; // Stop water flow.
                log::warn!("leak detected for the first time");
            } else {
                log::error!("leak detected multiple times");
            }
        }

        // NOTE: We send the normalized number of ticks (i.e., ticks per second) to the Cloud.
        if report_flow(&mut http, &Flow { addr, flow: unit }).await.map_err(|EspIOError(err)| err)? {
            log::info!("no shutdown request from the service after reporting ticks");
        } else {
            // We received a 503, we need to resume water flow.
            valve.lock().unwrap().set_high()?;
            log::warn!("remote shutdown requested by the Cloud");
        }

    }
}

use core::time::Duration;
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Input, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Flow, MacAddress};

use crate::{
    flow::take_ticks,
    http::{report_flow, report_leak, HttpClient}, SharedOutputPin,
};

pub async fn report<Tap: Pin, Valve: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    mut http: HttpClient,
    tap: PinDriver<'_, Tap, Input>,
    valve: SharedOutputPin<'_, Valve>,
) -> Result<(), EspError> {
    loop {
        timer.after(Duration::from_secs(5))?.await;
        let flow = take_ticks();
        log::info!("{flow} ticks detected since last reset");

        // TODO: In the future, we may want to piggy-back the leak detection
        // to the regular reporting instead. Not only is it more network-efficient,
        // but we also allow the Cloud to handle all leak-related logic.

        // Check if water is passing through while the tap is closed
        if tap.is_high() && flow > 100 {
            if report_leak(&mut http, &addr.0).await.map_err(|EspIOError(err)| err)? {
                log::warn!("leak detected for the first time");
            } else {
                log::error!("leak detected multiple times");
            }
        }

        if report_flow(&mut http, &Flow { addr, flow }).await.map_err(|EspIOError(err)| err)? {
            log::info!("no shutdown request from the service after reporting ticks");
            continue;
        }

        valve.lock().unwrap().set_low()?;
        log::warn!("remote shutdown requested by the Cloud");
    }
}

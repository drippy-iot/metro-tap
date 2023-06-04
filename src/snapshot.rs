use core::time::Duration;
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Input, Output, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Ping, MacAddress};

use crate::{
    flow::take_ticks,
    http::{ping, HttpClient},
    valve::ValveSystem,
};

pub async fn report<Tap: Pin, Valve: Pin, TapLed: Pin, ValveLed: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    mut http: HttpClient,
    tap: PinDriver<'_, Tap, Input>,
    mut tap_led: PinDriver<'_, TapLed, Output>,
    mut valve: ValveSystem<'_, Valve, ValveLed>,
) -> Result<(), EspError> {
    const SECONDS: u16 = 3;
    loop {
        timer.after(Duration::from_secs(SECONDS.into()))?.await;
        let flow = take_ticks();
        let unit = flow / SECONDS;
        log::info!("{flow} total ticks (i.e., {unit} ticks per second) detected since last reset");

        // Check if water is passing through while the tap is closed
        let leak = if tap.is_low() {
            tap_led.set_low()?;
            if flow > 10 {
                // TODO: Block actuation if we're in bypass mode.
                valve.stop_flow()?;
                log::warn!("leak detected");
                true
            } else {
                false
            }
        } else {
            tap_led.set_high()?;
            false
        };

        // NOTE: We send the normalized number of ticks (i.e., ticks per second) to the Cloud.
        if ping(&mut http, &Ping { addr, leak, flow: unit }).await.map_err(|EspIOError(err)| err)? {
            log::info!("no bypass request from the server");
        } else {
            valve.start_flow()?;
            log::warn!("remote shutdown requested by the Cloud");
        }
    }
}

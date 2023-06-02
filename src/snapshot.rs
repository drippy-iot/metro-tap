use core::time::Duration;
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Input, Output, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Flow, MacAddress};

use crate::{
    flow::take_ticks,
    http::{report_flow, report_leak, HttpClient}, SharedOutputPin,
};

pub async fn report<Tap: Pin, Valve: Pin, TapLed: Pin, ValveLed: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    mut http: HttpClient,
    tap: PinDriver<'_, Tap, Input>,
    mut tap_led: PinDriver<'_, TapLed, Output>,
    valve: SharedOutputPin<'_, Valve>,
    valve_led: SharedOutputPin<'_, ValveLed>,
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
        if tap.is_low() {
            tap_led.set_low()?;
            if flow > 10 {
                if report_leak(&mut http, &addr.0).await.map_err(|EspIOError(err)| err)? {
                    log::warn!("leak detected for the first time");
                    valve.lock().unwrap().set_low()?; // Stop water flow.
                    valve_led.lock().unwrap().set_high()?; // Turn on the alarm LED.
                } else {
                    log::error!("leak detected multiple times");
                }
            }
        } else {
            tap_led.set_high()?;
        }

        // NOTE: We send the normalized number of ticks (i.e., ticks per second) to the Cloud.
        if report_flow(&mut http, &Flow { addr, flow: unit }).await.map_err(|EspIOError(err)| err)? {
            log::info!("no shutdown request from the service after reporting ticks");
            continue;
        }

        valve.lock().unwrap().set_high()?; // We received a 503, we need to resume water flow.
        valve_led.lock().unwrap().set_low()?; // Turn off the alarm LED.
        log::warn!("remote shutdown requested by the Cloud");
    }
}

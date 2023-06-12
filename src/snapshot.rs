use core::sync::atomic::{AtomicBool, Ordering};
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Input, Output, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{
    report::{Ping, POLL_QUANTUM},
    MacAddress,
};
use std::sync::{Arc, Mutex};

use crate::{
    flow::take_ticks,
    http::{ping, Command, HttpClient},
    valve::ValveSystem,
};

pub async fn report<Tap: Pin, Valve: Pin, TapLed: Pin, ValveLed: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    http: Arc<Mutex<HttpClient>>,
    should_bypass: Arc<AtomicBool>,
    tap: PinDriver<'_, Tap, Input>,
    mut tap_led: PinDriver<'_, TapLed, Output>,
    mut valve: ValveSystem<'_, Valve, ValveLed>,
) -> Result<(), EspError> {
    let mut allowance = 0u8;
    loop {
        timer.after(POLL_QUANTUM)?.await;
        let flow = take_ticks();
        let unit = u64::from(flow) / POLL_QUANTUM.as_secs();
        log::info!("{flow} total ticks (i.e., {unit} ticks per second) detected since last reset");

        // Check if water is passing through while the tap is closed
        let leak = 'detect: {
            if tap.is_high() {
                tap_led.set_high()?;
                allowance = 0;
                break 'detect false;
            }

            tap_led.set_low()?;
            if flow <= 10 {
                break 'detect false;
            }

            if should_bypass.load(Ordering::Relaxed) {
                valve.start_flow()?;
                log::warn!("leak detected but bypassed");
                break 'detect true;
            }

            // NOTE: Threshold of 1 is technically just a `bool`.
            allowance += 1;
            if allowance <= 1 {
                log::warn!("leak detected, within allowance");
                break 'detect false;
            }

            valve.stop_flow()?;
            log::warn!("leak detected and valve actuated");
            false
        };

        // NOTE: We send the normalized number of ticks (i.e., ticks per second) to the Cloud.
        let mut guard = http.lock().unwrap();
        let command = ping(&mut guard, &Ping { addr, leak, flow: unit.try_into().unwrap() })
            .await
            .map_err(|EspIOError(err)| err)?;
        drop(guard);

        match command {
            Command::None => log::info!("no command issued from the server"),
            Command::Open => {
                should_bypass.store(true, Ordering::Relaxed);
                valve.start_flow()?;
                allowance = 0;
                log::warn!("server issued a remote bypass");
            }
            Command::Close => {
                valve.stop_flow()?;
                allowance = 0;
                log::warn!("server issued a remote shutdown");
            }
        }
    }
}

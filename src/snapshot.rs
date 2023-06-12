use core::sync::atomic::{AtomicBool, Ordering, AtomicU8};
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

static ALLOWANCE: AtomicU8 = AtomicU8::new(0);

pub async fn report<Tap: Pin, Valve: Pin, TapLed: Pin, ValveLed: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    http: Arc<Mutex<HttpClient>>,
    should_bypass: Arc<AtomicBool>,
    tap: PinDriver<'_, Tap, Input>,
    mut tap_led: PinDriver<'_, TapLed, Output>,
    mut valve: ValveSystem<'_, Valve, ValveLed>,
) -> Result<(), EspError> {
    loop {
        timer.after(POLL_QUANTUM)?.await;
        let flow = take_ticks();
        let unit = u64::from(flow) / POLL_QUANTUM.as_secs();
        log::info!("{flow} total ticks (i.e., {unit} ticks per second) detected since last reset");

        // Check if water is passing through while the tap is closed
        let leak = if tap.is_low() {
            tap_led.set_low()?;
            if flow > 10 {
                if should_bypass.load(Ordering::Relaxed) {
                    valve.start_flow()?;
                    log::warn!("leak detected but bypassed");
                    true
                } else {
                    if ALLOWANCE.fetch_add(1, Ordering::Relaxed) > 0 {
                        valve.stop_flow()?;
                        log::warn!("leak detected and valve actuated");
                        true
                    } else {
                        log::warn!("leak detected, within allowance");
                        false
                    }
                }
            } else {
                false
            }
        } else {
            tap_led.set_high()?;
            ALLOWANCE.swap(0, Ordering::Relaxed);
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
                ALLOWANCE.swap(0, Ordering::Relaxed);
                valve.start_flow()?;
                log::warn!("server issued a remote bypass");
            }
            Command::Close => {
                ALLOWANCE.swap(0, Ordering::Relaxed);
                valve.stop_flow()?;
                log::warn!("server issued a remote shutdown");
            }
        }
    }
}

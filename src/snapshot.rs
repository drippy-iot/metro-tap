use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Input, Output, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Ping, MacAddress};
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
        let mut guard = http.lock().unwrap();
        let command = ping(&mut guard, &Ping { addr, leak, flow: unit }).await.map_err(|EspIOError(err)| err)?;
        drop(guard);

        match command {
            Command::None => log::info!("no command issued from the server"),
            Command::Open => {
                should_bypass.store(true, Ordering::Relaxed);
                valve.start_flow()?;
                log::warn!("server issued a remote bypass");
            }
            Command::Close => {
                valve.stop_flow()?;
                log::warn!("server issued a remote shutdown");
            }
        }
    }
}

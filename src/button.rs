use core::sync::atomic::{AtomicBool, Ordering};
use esp_idf_hal::gpio::{Input, Pin, PinDriver};
use esp_idf_svc::errors::EspIOError;
use esp_idf_sys::EspError;
use std::sync::{Arc, Mutex};

use crate::http::{bypass, HttpClient};

pub async fn deactivate_bypass_mode<Button: Pin>(
    http: Arc<Mutex<HttpClient>>,
    should_bypass: Arc<AtomicBool>,
    mut button: PinDriver<'_, Button, Input>,
) -> Result<(), EspError> {
    loop {
        button.wait_for_falling_edge().await.map_err(|err| err.cause())?;
        if should_bypass.swap(false, Ordering::Relaxed) {
            let mut guard = http.lock().unwrap();
            bypass(&mut guard).await.map_err(|EspIOError(err)| err)?;
            log::info!("undid bypass mode");
        } else {
            log::warn!("system is already in normal operation");
        }
    }
}

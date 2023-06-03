use crate::valve::ValveSystem;

use esp_idf_hal::gpio::{GpioError, Input, Pin, PinDriver};
use std::sync::{Arc, Mutex};

pub async fn bypass<Button: Pin, Valve: Pin, Led: Pin>(
    mut button: PinDriver<'_, Button, Input>,
    valve: Arc<Mutex<ValveSystem<'_, Valve, Led>>>,
) -> Result<(), GpioError> {
    loop {
        button.wait_for_falling_edge().await?;
        valve.lock().unwrap().start_flow()?;
        log::warn!("manual bypass requested by the reset button");
    }
}

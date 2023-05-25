use esp_idf_hal::gpio::{GpioError, Input, Pin, PinDriver};

use crate::SharedOutputPin;

pub async fn bypass<Button: Pin, Valve: Pin>(
    mut button: PinDriver<'_, Button, Input>,
    valve: SharedOutputPin<'_, Valve>,
) -> Result<(), GpioError> {
    loop {
        button.wait_for_falling_edge().await?;
        valve.lock().unwrap().set_high()?; // Manual bypass should allow water to flow.
        log::warn!("manual bypass requested by the reset button");
    }
}

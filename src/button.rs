use esp_idf_hal::gpio::{GpioError, Input, Pin, PinDriver};
use esp_idf_svc::errors::EspIOError;
use model::MacAddress;

use crate::{
    http::{report_reset, HttpClient},
    SharedOutputPin,
};

pub async fn bypass<Button: Pin, Valve: Pin>(
    mac: MacAddress,
    mut http: HttpClient,
    mut button: PinDriver<'_, Button, Input>,
    valve: SharedOutputPin<'_, Valve>,
) -> Result<(), GpioError> {
    loop {
        button.wait_for_falling_edge().await?;
        if report_reset(&mut http, &mac.0).await.map_err(|EspIOError(err)| err)? {
            // Manual bypass should allow water to flow.
            valve.lock().unwrap().set_high()?;
            log::info!("successfully reported a manual bypass to the cloud");
        } else {
            log::warn!("bypass requested when valve is already open to begin with");
        }
    }
}

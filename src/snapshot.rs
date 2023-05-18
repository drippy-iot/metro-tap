use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_svc::timer::EspTimer;
use esp_idf_sys::EspError;

use crate::{
    flow::take_ticks,
    http::{report_to_server, HttpClient},
};

pub async fn tick(mut timer: AsyncTimer<EspTimer>, mut http: HttpClient) -> Result<(), EspError> {
    loop {
        timer.after(core::time::Duration::from_secs(5))?.await;
        let ticks = take_ticks();
        log::info!("{ticks} ticks detected since last reset");

        let bytes = report_to_server(&mut http).await?;
        let html = String::from_utf8(bytes).unwrap();
        log::info!("{html}");
    }
}

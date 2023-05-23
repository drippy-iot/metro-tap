use core::time::Duration;
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{Output, Pin, PinDriver};
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Flow, MacAddress};

use crate::{
    flow::take_ticks,
    http::{report_flow, HttpClient},
};

pub async fn tick<T: Pin>(
    addr: MacAddress,
    mut timer: AsyncTimer<EspTimer>,
    mut http: HttpClient,
    mut valve: PinDriver<'_, T, Output>,
) -> Result<(), EspError> {
    loop {
        timer.after(Duration::from_secs(5))?.await;
        let flow = take_ticks();
        log::info!("{flow} ticks detected since last reset");

        if report_flow(&mut http, &Flow { addr, flow }).await.map_err(|EspIOError(err)| err)? {
            log::info!("no shutdown request from the service after reporting ticks");
            continue;
        }

        // FIXME: For now, we treat each shutdown request as a toggle.
        log::warn!("remote bypass requested by the Cloud");
        valve.toggle()?;
    }
}

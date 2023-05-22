use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_svc::{errors::EspIOError, timer::EspTimer};
use esp_idf_sys::EspError;
use model::{report::Flow, MacAddress};

use crate::{
    flow::take_ticks,
    http::{report_flow, HttpClient},
};

pub async fn tick(mut timer: AsyncTimer<EspTimer>, mut http: HttpClient, addr: MacAddress) -> Result<(), EspError> {
    loop {
        timer.after(core::time::Duration::from_secs(5))?.await;
        let flow = take_ticks();
        log::info!("{flow} ticks detected since last reset");

        if report_flow(&mut http, &Flow { addr, flow }).await.map_err(|EspIOError(err)| err)? {
            log::info!("no shutdown request from the service after reporting ticks");
            continue;
        }

        log::warn!("leak reported by the Cloud");
        // TODO: actuate the valves
    }
}

mod net;

use embedded_svc::wifi;
use esp_idf_hal::{peripherals::Peripherals, task::executor::EspExecutor};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    timer::EspTimerService,
    wifi::{AsyncWifi, EspWifi},
};
use esp_idf_sys::{self as _, EspError};

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let timer_svc = EspTimerService::new()?;

    let Peripherals { modem, .. } = Peripherals::take().ok_or(EspError::from_infallible::<-1>())?;
    let wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = AsyncWifi::wrap(wifi, sysloop, timer_svc)?;
    wifi.set_configuration(&wifi::Configuration::Client(Default::default()))?;

    let rt = EspExecutor::<16, _>::new();
    rt.spawn_local(async {
        net::init(&mut wifi).await?;
        Ok::<_, EspError>(())
    })
    .unwrap()
    .detach();
    rt.run(|| true);

    Ok(())
}

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
        wifi.start().await?;
        log::info!("Wi-Fi started");

        for wifi::AccessPointInfo { ssid, signal_strength, auth_method, .. } in wifi.scan().await? {
            let Some(name) = ssid.strip_prefix("DRIPPY_") else {
                log::warn!("skipping {ssid} [{signal_strength}]");
                continue;
            };

            log::info!("connecting to {name}");
            wifi.set_configuration(&wifi::Configuration::Client(wifi::ClientConfiguration {
                ssid,
                auth_method,
                password: "drippy-test".into(),
                ..Default::default()
            }))?;
            wifi.connect().await?;
            log::info!("successfully connected to network");

            wifi.wait_netif_up().await?;
            break;
        }

        let netif = wifi.wifi().sta_netif();
        let info = netif.get_ip_info()?;
        log::info!("connected to interface {info:?}");

        Ok::<_, EspError>(())
    })
    .unwrap()
    .detach();
    rt.run(|| true);

    Ok(())
}

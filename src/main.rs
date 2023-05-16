use esp_idf_sys::{self as _, EspError};

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    use esp_idf_hal::peripherals::Peripherals;
    let Peripherals { modem, .. } = Peripherals::take().ok_or(EspError::from_infallible::<-1>())?;

    use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    use embedded_svc::wifi::{ClientConfiguration, Configuration};
    let mut wifi = EspWifi::new(modem, sys_loop, Some(nvs))?;

    log::info!("Setting Wi-Fi default configuration...");
    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    log::info!("Starting Wi-Fi...");
    wifi.start()?;

    'connect: loop {
        use embedded_svc::wifi::AccessPointInfo;
        log::info!("Starting a new round of scanning...");
        for AccessPointInfo { ssid, signal_strength, auth_method, .. } in wifi.scan()? {
            let Some(name) = ssid.strip_prefix("DRIPPY_") else {
                log::info!("Skipping {ssid} [{signal_strength}]...");
                continue;
            };

            log::info!("Attempting to connect to {name} [{signal_strength}]...");
            wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid,
                auth_method,
                password: "drippy-test".into(),
                ..Default::default()
            }))?;
            wifi.connect()?;

            break 'connect;
        }
    }

    use embedded_svc::ipv4::IpInfo;
    let IpInfo { ip, subnet, .. } = wifi.ap_netif().get_ip_info()?;
    log::info!("Now connected as {ip} in subnet {subnet}.");

    Ok(())
}

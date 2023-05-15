use esp_idf_sys::{self as _, EspError};

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    use esp_idf_hal::peripherals::Peripherals;
    let Peripherals { modem, .. } = Peripherals::take().ok_or(EspError::from_infallible::<-1>())?;

    use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
    let mut wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;

    let conf = Configuration::Client(ClientConfiguration {
        ssid: "SSID".into(),
        password: "PASSWORD".into(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    });

    println!("Setting Wi-Fi configuration...");
    wifi.set_configuration(&conf)?;

    println!("Starting Wi-Fi...");
    wifi.start()?;

    println!("Connecting Wi-Fi...");
    wifi.connect()?;

    // TODO: TCP Listener or whatever...

    Ok(())
}

mod button;
mod flow;

use embedded_svc::{
    ipv4::IpInfo,
    utils::asyncify::timer::AsyncTimerService,
    wifi::{AccessPointInfo, ClientConfiguration, Configuration},
};
use esp_idf_hal::{
    gpio::{PinDriver, Pins, Pull},
    peripherals::Peripherals,
    task::executor::{EspExecutor, Local},
};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, timer::EspTimerService, wifi::EspWifi};
use esp_idf_sys::{self as _, EspError};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let Peripherals { modem, pins: Pins { gpio18: tap_toggle, gpio23: flow_sensor, .. }, .. } =
        Peripherals::take().ok_or(EspError::from_infallible::<-1>())?;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut timer_svc = AsyncTimerService::new(EspTimerService::new()?);
    let timer = timer_svc.timer()?;
    drop(timer_svc);

    let mut wifi = EspWifi::new(modem, sys_loop, Some(nvs))?;

    log::info!("Setting Wi-Fi default configuration...");
    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    log::info!("Starting Wi-Fi...");
    wifi.start()?;

    'connect: loop {
        log::info!("Starting a new round of scanning...");

        for AccessPointInfo { ssid, signal_strength, auth_method, .. } in wifi.scan()? {
            let Some(name) = ssid.strip_prefix("DRIPPY_") else {
                log::warn!("Skipping {ssid} [{signal_strength}]...");
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

    let IpInfo { ip, subnet, .. } = wifi.ap_netif().get_ip_info()?;
    log::info!("Now connected as {ip} in subnet {subnet}.");

    let exec = EspExecutor::<16, Local>::new();

    let flow = PinDriver::input(flow_sensor)?;
    exec.spawn(flow::detect(flow))?.detach();

    let mut faucet_button = PinDriver::input(tap_toggle)?;
    faucet_button.set_pull(Pull::Up)?;
    exec.spawn(button::tap_toggle(timer, faucet_button))?.detach();

    exec.run(|| true);
    Ok(())
}

mod button;
mod flow;
mod http;
mod snapshot;

use embedded_svc::{
    http::client::asynch::{Client as HttpClient, TrivialUnblockingConnection},
    ipv4::IpInfo,
    utils::asyncify::Asyncify,
    wifi::{AccessPointInfo, ClientConfiguration, Configuration},
};
use esp_idf_hal::{
    gpio::{PinDriver, Pins, Pull},
    peripherals::Peripherals,
    task::executor::EspExecutor,
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, http::client::EspHttpConnection, nvs::EspDefaultNvsPartition,
    timer::EspTimerService, wifi::EspWifi,
};
use esp_idf_sys::{self as _, EspError};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    // Get the peripherals from the board
    let Peripherals {
        modem,
        pins:
            Pins {
                gpio18: tap_pin,
                gpio19: metro_valve_pin,
                gpio21: tap_valve_pin,
                gpio22: bypass_pin,
                gpio23: flow_sensor_pin,
                ..
            },
        ..
    } = Peripherals::take().ok_or(EspError::from_infallible::<-1>())?;

    // Set up the input pins
    let flow_sensor = PinDriver::input(flow_sensor_pin)?;
    let mut tap = PinDriver::input(tap_pin)?;
    tap.set_pull(Pull::Up)?;
    let mut bypass = PinDriver::input(bypass_pin)?;
    bypass.set_pull(Pull::Up)?;

    // Set up the ouput pins
    let metro_valve = PinDriver::output(metro_valve_pin)?;
    let tap_valve = PinDriver::output(tap_valve_pin)?;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut timer_svc = EspTimerService::new()?.into_async();
    let tap_timer = timer_svc.timer()?;
    let snapshot_timer = timer_svc.timer()?;
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

    let http = EspHttpConnection::new(&Default::default())?;
    let http = TrivialUnblockingConnection::new(http);
    let http = HttpClient::wrap(http);

    let exec = EspExecutor::<16, _>::new();
    exec.spawn(flow::detect(flow_sensor))?.detach();
    exec.spawn(button::tap::toggle(tap_timer, tap))?.detach();
    exec.spawn_local(snapshot::tick(snapshot_timer, http))?.detach();
    exec.run(|| true);

    Ok(())
}

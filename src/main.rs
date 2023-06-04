mod flow;
mod http;
mod net;
mod snapshot;
mod valve;

use embedded_svc::{http::client::asynch::TrivialUnblockingConnection, utils::asyncify::Asyncify as _, wifi};
use esp_idf_hal::{
    gpio::{PinDriver, Pins},
    peripherals::Peripherals,
    task::executor::EspExecutor,
};
use esp_idf_svc::{
    errors::EspIOError,
    eventloop::EspSystemEventLoop,
    http::client::EspHttpConnection,
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

    // NOTE: GPIO 22 used to be the manual bypass button in the hardware (for debugging).
    let Peripherals {
        modem,
        pins:
            Pins {
                gpio21: tap_sensor_pin,
                gpio23: valve_pin,
                gpio34: flow_sensor_pin,
                gpio33: tap_led_pin,
                gpio32: valve_led_pin,
                ..
            },
        ..
    } = Peripherals::take().ok_or_else(EspError::from_infallible::<-1>)?;

    // Set up pins
    let valve = PinDriver::output(valve_pin)?;
    let tap = PinDriver::input(tap_sensor_pin)?;
    let tap_led = PinDriver::output(tap_led_pin)?;
    let valve_led = PinDriver::output(valve_led_pin)?;
    let flow = PinDriver::input(flow_sensor_pin)?;

    // Allow the water to flow
    let mut valve = valve::ValveSystem { control: valve, led: valve_led };
    valve.start_flow()?;

    // Initialize other services
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let timer_svc = EspTimerService::new()?;

    // Set up Wi-Fi driver
    let wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = AsyncWifi::wrap(wifi, sysloop, timer_svc.clone())?;
    wifi.set_configuration(&wifi::Configuration::Client(Default::default()))?;

    // Set up asynchronous timer
    let mut timer_svc = timer_svc.into_async();
    let timer = timer_svc.timer()?;

    // Set up asynchronous HTTP service
    let conn = EspHttpConnection::new(&Default::default())?;
    let conn = TrivialUnblockingConnection::new(conn);
    let mut http = http::HttpClient::wrap(conn);

    let exec = EspExecutor::<4, _>::new();
    exec.spawn_local_detached(async {
        let mac = net::init(&mut wifi).await?;
        http::register_to_server(&mut http, &mac.0).await.map_err(|EspIOError(err)| err)?;
        exec.spawn_detached(flow::detect(flow))
            .unwrap()
            .spawn_local_detached(snapshot::report(mac, timer, http, tap, tap_led, valve))
            .unwrap();
        Ok::<_, EspError>(())
    })
    .unwrap();
    exec.run(|| true);

    Ok(())
}

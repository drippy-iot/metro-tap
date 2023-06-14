mod button;
mod flow;
mod http;
mod net;
mod snapshot;
mod valve;

use core::sync::atomic::AtomicBool;
use embedded_svc::{http::client::asynch::TrivialUnblockingConnection, utils::asyncify::Asyncify as _, wifi};
use esp_idf_hal::{
    gpio::{PinDriver, Pins},
    peripherals::Peripherals,
    task::executor::EspExecutor,
};
use esp_idf_svc::{
    errors::EspIOError,
    eventloop::EspSystemEventLoop,
    http::client::{Configuration as HttpConfig, EspHttpConnection, FollowRedirectsPolicy},
    nvs::EspDefaultNvsPartition,
    timer::EspTimerService,
    wifi::{AsyncWifi, EspWifi},
};
use esp_idf_sys::{self as _, EspError};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let Peripherals {
        modem,
        pins:
            Pins {
                gpio2: ready_led_pin,
                gpio21: tap_sensor_pin,
                gpio22: bypass_button_pin,
                gpio23: valve_pin,
                gpio34: flow_sensor_pin,
                gpio33: tap_led_pin,
                gpio32: valve_led_pin,
                ..
            },
        ..
    } = Peripherals::take().ok_or_else(EspError::from_infallible::<-1>)?;

    // Set up pins
    let mut ready_led = PinDriver::output(ready_led_pin)?;
    let valve = PinDriver::output(valve_pin)?;
    let tap = PinDriver::input(tap_sensor_pin)?;
    let mut bypass_button = PinDriver::input(bypass_button_pin)?;
    let tap_led = PinDriver::output(tap_led_pin)?;
    let valve_led = PinDriver::output(valve_led_pin)?;
    let flow = PinDriver::input(flow_sensor_pin)?;

    // Set the pull mode of the pins
    bypass_button.set_pull(esp_idf_hal::gpio::Pull::Up)?;

    // Allow the water to flow
    let mut valve = valve::ValveSystem { control: valve, led: valve_led };
    valve.start_flow()?; // this will be re-initialized later on registration

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
    let cert = include_bytes!("_.up.railway.app.crt");
    unsafe { esp_idf_sys::esp_tls_set_global_ca_store(cert.as_ptr().cast(), cert.len() as u32) };
    let conn = EspHttpConnection::new(&HttpConfig {
        follow_redirects_policy: FollowRedirectsPolicy::FollowAll,
        use_global_ca_store: true,
        ..Default::default()
    })?;
    let conn = TrivialUnblockingConnection::new(conn);
    let mut http = http::HttpClient::wrap(conn);

    let exec = EspExecutor::<4, _>::new();
    exec.spawn_local_detached(async {
        // Initialize Wi-Fi
        let mac = net::init(&mut wifi).await?;

        // Initialize the pipe state
        let command = http::register_to_server(&mut http, &mac.0).await.map_err(|EspIOError(err)| err)?;
        let init = match command {
            http::Command::None => {
                log::info!("no pending commands from the server");
                false
            }
            http::Command::Close => {
                log::info!("server initialized the system with a close command");
                valve.stop_flow()?;
                false
            }
            http::Command::Open => {
                log::info!("server initialized the system with an open command");
                valve.start_flow()?;
                true
            }
        };
        let bypass = Arc::new(AtomicBool::new(init));

        // Turn on the LED to find out whether
        ready_led.set_high()?;

        let http = Arc::new(Mutex::new(http));
        exec.spawn_detached(flow::detect(flow))
            .unwrap()
            .spawn_local_detached(button::deactivate_bypass_mode(mac, http.clone(), bypass.clone(), bypass_button))
            .unwrap()
            .spawn_local_detached(snapshot::report(mac, timer, http, bypass, tap, tap_led, valve))
            .unwrap();
        Ok::<_, EspError>(())
    })
    .unwrap();
    exec.run(|| true);

    Ok(())
}

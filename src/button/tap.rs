use core::{sync::atomic::{AtomicBool, Ordering}, time::Duration};
use embedded_svc::utils::asyncify::timer::AsyncTimer;
use esp_idf_hal::gpio::{GpioError, Input, Pin as GpioPin, PinDriver};
use esp_idf_svc::timer::EspTimer;

static TAP: AtomicBool = AtomicBool::new(false);

/// The pin must be a pull-up input.
pub async fn toggle<T: GpioPin>(
    mut timer: AsyncTimer<EspTimer>,
    mut pin: PinDriver<'_, T, Input>,
) -> Result<(), GpioError> {
    TAP.store(pin.is_high(), Ordering::Relaxed);
    loop {
        pin.wait_for_rising_edge().await?;
        let prev = TAP.fetch_xor(true, Ordering::Relaxed); // poor man's toggle
        log::info!("previous value for faucet is {prev}");

        // Three-second debounce is necessary so that the solenoid valve doesn't break.
        const DEBOUNCE: Duration = Duration::from_secs(3);
        timer.after(DEBOUNCE).unwrap().await;
    }
}

/// Checks if the faucet has been toggled on.
pub fn is_on() -> bool {
    TAP.load(Ordering::Relaxed)
}

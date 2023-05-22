use core::sync::atomic::{AtomicU16, Ordering};
use esp_idf_hal::gpio::{GpioError, Input, Pin as GpioPin, PinDriver};

static TICKS: AtomicU16 = AtomicU16::new(0);

pub fn take_ticks() -> u16 {
    log::debug!("taking the ticks to 0");
    TICKS.swap(0, Ordering::Relaxed)
}

/// Infinitely reacts to the rising edge of the flow sensor.
pub async fn detect<T: GpioPin>(mut pin: PinDriver<'_, T, Input>) -> Result<(), GpioError> {
    loop {
        // NOTE: We do not guard against integer overflow, but we do expect
        // the counter to be reset every now and then by the timer interrupt.
        pin.wait_for_rising_edge().await?;
        log::debug!("flow sensor tick detected");
        TICKS.fetch_add(1, Ordering::Relaxed);
    }
}

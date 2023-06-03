use esp_idf_hal::gpio::{Output, Pin, PinDriver};
use esp_idf_sys::EspError;

pub struct ValveSystem<'p, Control: Pin, Led: Pin> {
    pub control: PinDriver<'p, Control, Output>,
    pub led: PinDriver<'p, Led, Output>,
}

impl<Control: Pin, Led: Pin> ValveSystem<'_, Control, Led> {
    pub fn start_flow(&mut self) -> Result<(), EspError> {
        self.control.set_high()?;
        self.led.set_low()
    }

    pub fn stop_flow(&mut self) -> Result<(), EspError> {
        self.control.set_low()?;
        self.led.set_high()
    }
}

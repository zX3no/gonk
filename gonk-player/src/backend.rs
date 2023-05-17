use crate::decoder::Symphonia;
use std::error::Error;

#[cfg(windows)]
pub use crate::wasapi::*;

#[cfg(unix)]
pub use crate::pipewire::*;

pub trait Backend {
    fn sample_rate(&self) -> usize;
    fn set_sample_rate(&mut self, sample_rate: usize, device: &Device);
    fn fill_buffer(&mut self, volume: f32, symphonia: &mut Symphonia)
        -> Result<(), Box<dyn Error>>;
    //TODO: Move devices() and default_devices() into the backend?
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    #[cfg(windows)]
    pub inner: *mut winapi::um::mmdeviceapi::IMMDevice,
    pub name: String,
}

pub fn new(device: &Device, sample_rate: Option<usize>) -> Box<dyn Backend> {
    #[cfg(windows)]
    return Box::new(Wasapi::new(device, sample_rate));

    #[cfg(unix)]
    return Box::new(PipeWire::new(device, sample_rate));
}

//TODO: Remove?
pub fn devices() -> Vec<Device> {
    #[cfg(windows)]
    return unsafe { DEVICES.to_vec() };

    #[cfg(unix)]
    return todo!();
}

//TODO: Remove
pub unsafe fn default_device() -> &'static Device {
    #[cfg(windows)]
    return DEFAULT_DEVICE.as_ref().unwrap();

    #[cfg(unix)]
    return todo!();
}

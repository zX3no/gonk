use crate::decoder::Symphonia;

#[cfg(windows)]
pub use crate::wasapi::*;

#[cfg(unix)]
pub use crate::pipewire::*;

pub trait Backend {
    fn sample_rate(&self) -> usize;
    fn set_sample_rate(&mut self, sample_rate: usize, device: &Device);
    fn fill_buffer(&mut self, volume: f32, symphonia: &mut Symphonia);
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Device {
    #[cfg(windows)]
    pub inner: *mut winapi::um::mmdeviceapi::IMMDevice,
    pub name: String,
}

pub fn new(device: &Device) -> Box<dyn Backend> {
    #[cfg(windows)]
    return Box::new(Wasapi::new(device, None));

    #[cfg(unix)]
    return Box::new(PipeWire::new(device, None));
}

pub fn devices() -> &'static [Device] {
    #[cfg(windows)]
    return unsafe { &DEVICES };

    #[cfg(unix)]
    return todo!();
}

pub fn default_device() -> Option<&'static Device> {
    #[cfg(windows)]
    return unsafe { DEFAULT_DEVICE.as_ref() };

    #[cfg(unix)]
    return todo!();
}

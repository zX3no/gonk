use crate::backend::{Backend, Device};
use std::error::Error;

pub struct PipeWire {}

impl PipeWire {
    pub fn new(device: &Device, sample_rate: Option<usize>) -> Self {
        Self {}
    }
}

impl Backend for PipeWire {
    fn sample_rate(&self) -> usize {
        todo!()
    }

    fn set_sample_rate(&mut self, sample_rate: usize, device: &Device) {
        todo!()
    }

    fn fill_buffer(
        &mut self,
        volume: f32,
        symphonia: &mut crate::decoder::Symphonia,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}

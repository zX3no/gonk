use crate::backend::{Backend, Device};

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

    fn set_sample_rate(&mut self, sample_rate: usize, device: &Device) -> usize {
        todo!()
    }

    fn fill_buffer(&self, volume: f32, decoder: &mut crate::decoder::Symphonia) {
        todo!()
    }
}

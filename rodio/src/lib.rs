#![cfg_attr(test, deny(missing_docs))]
use cpal::traits::HostTrait;
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
use decoder::Decoder;

mod conversions;
mod sink;
mod stream;

pub mod buffer;
pub mod decoder;
pub mod dynamic_mixer;
pub mod queue;
pub mod source;

pub use crate::conversions::Sample;
pub use crate::sink::Sink;
pub use crate::source::Source;
pub use crate::stream::{OutputStream, OutputStreamHandle, PlayError, StreamError};

use std::fs::File;
use std::path::Path;
use std::time::Duration;

static VOLUME_STEP: u16 = 5;

pub struct Player {
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    total_duration: Option<Duration>,
    volume: u16,
    safe_guard: bool,
}

impl Default for Player {
    fn default() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        let volume = 15;
        sink.set_volume(volume as f32 / 1000.0);

        Self {
            stream,
            handle,
            sink,
            total_duration: None,
            volume,
            safe_guard: true,
        }
    }
}

impl Player {
    #[must_use]
    pub fn volume(mut self, volume: u16) -> Self {
        self.volume = volume;
        self
    }
    pub fn change_volume(&mut self, positive: bool) {
        if positive {
            self.volume += VOLUME_STEP;
        } else if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        if self.volume > 100 {
            self.volume = 100;
        }

        self.sink.set_volume(self.volume as f32 / 1000.0);
    }
    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }
    pub fn play(&mut self, path: &Path) {
        //TODO: if the volume is zero the song will play really fast???
        self.stop();
        let file = File::open(path).unwrap();
        let decoder = Decoder::new(file).unwrap();
        self.total_duration = decoder.total_duration();
        self.sink.append(decoder);
    }
    pub fn stop(&mut self) {
        self.sink.destroy();
        self.sink = Sink::try_new(&self.handle).unwrap();
        self.sink.set_volume(self.volume as f32 / 1000.0);
    }
    pub fn elapsed(&self) -> Duration {
        self.sink.elapsed()
    }
    pub fn duration(&self) -> Option<f64> {
        //TODO: wtf is this?
        self.total_duration
            .map(|duration| duration.as_secs_f64() - 0.29)
    }
    pub fn toggle_playback(&self) {
        self.sink.toggle_playback();
    }
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
    pub fn seek_fw(&mut self) {
        let seek = self.elapsed().as_secs_f64() + 10.0;
        if let Some(duration) = self.duration() {
            if seek > duration {
                self.safe_guard = true;
            } else {
                self.seek_to(Duration::from_secs_f64(seek));
            }
        }
    }
    pub fn seek_bw(&mut self) {
        let mut seek = self.elapsed().as_secs_f64() - 10.0;
        if seek < 0.0 {
            seek = 0.0;
        }

        self.seek_to(Duration::from_secs_f64(seek));
    }
    pub fn seek_to(&self, time: Duration) {
        self.sink.seek(time);
    }
    pub fn seeker(&self) -> f64 {
        if let Some(duration) = self.duration() {
            let elapsed = self.elapsed();
            elapsed.as_secs_f64() / duration
        } else {
            0.0
        }
    }
    pub fn trigger_next(&mut self) -> bool {
        if let Some(duration) = self.duration() {
            if self.elapsed().as_secs_f64() > duration {
                self.safe_guard = true;
            }
        }

        if self.safe_guard {
            self.safe_guard = false;
            true
        } else {
            false
        }
    }
    pub fn volume_percent(&self) -> u16 {
        self.volume
    }
    pub fn output_devices() -> Vec<Device> {
        let host_id = cpal::default_host().id();
        let host = cpal::host_from_id(host_id).unwrap();

        let mut devices: Vec<_> = host.output_devices().unwrap().collect();
        devices.sort_by_key(|a| a.name().unwrap().to_lowercase());
        devices
    }
    pub fn default_device() -> Option<Device> {
        cpal::default_host().default_output_device()
    }
    pub fn change_output_device(&mut self, device: &Device) {
        self.stop();
        let (stream, handle) = OutputStream::try_from_device(device).unwrap();
        self.stream = stream;
        self.handle = handle;
    }
}

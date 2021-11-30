#![cfg_attr(test, deny(missing_docs))]
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};

mod conversions;
mod sink;
mod stream;

pub mod buffer;
pub mod decoder;
pub mod dynamic_mixer;
pub mod queue;
pub mod source;

pub use crate::conversions::Sample;
pub use crate::decoder::Decoder;
pub use crate::sink::Sink;
pub use crate::source::Source;
pub use crate::stream::{OutputStream, OutputStreamHandle, PlayError, StreamError};

use std::path::Path;
use std::time::Duration;
use std::{fs::File, io::BufReader};

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    total_duration: Option<Duration>,
    volume: f32,
}
impl Player {
    pub fn new() -> Self {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        let volume = 0.01;
        sink.set_volume(volume);

        Self {
            _stream,
            handle,
            sink,
            total_duration: None,
            volume,
        }
    }
    pub fn set_volume(&mut self, v: f32) {
        //TODO: clamping
        self.volume += v;
        if self.volume < 0.0 {
            self.volume = 0.0;
        } else if self.volume > 0.1 {
            self.volume = 0.1;
        }
        self.sink.set_volume(self.volume);
    }
    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }
    pub fn play(&mut self, path: &Path) {
        self.stop();
        let file = File::open(path).unwrap();
        let decoder = Decoder::new(BufReader::new(file)).unwrap();
        self.total_duration = decoder.total_duration();
        self.sink.append(decoder);
    }
    pub fn stop(&mut self) {
        self.sink.drop();
        self.sink = Sink::try_new(&self.handle).unwrap();
        self.sink.set_volume(self.volume);
    }
    pub fn elapsed(&self) -> Duration {
        //TODO: change to option duration
        self.sink.elapsed()
    }
    pub fn duration(&self) -> Option<Duration> {
        //TODO: this is off by a second ?
        self.total_duration
    }
    pub fn toggle_playback(&self) {
        self.sink.toggle_playback();
    }
    pub fn seek_fw(&mut self) {
        let seek = self.elapsed().as_secs_f64() + 10.0;
        if let Some(duration) = self.duration() {
            if seek > duration.as_secs_f64() {
                self.stop()
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
    fn seek_to(&self, time: Duration) {
        self.sink.seek(time);
    }
    pub fn seeker(&self) -> f64 {
        if let Some(duration) = self.duration() {
            let elapsed = self.elapsed();
            elapsed.as_secs_f64() / duration.as_secs_f64()
        } else {
            0.0
        }
    }
    pub fn is_done(&self) -> bool {
        if let Some(duration) = self.duration() {
            if self.elapsed().as_secs_f64() + 1.0 > duration.as_secs_f64() {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

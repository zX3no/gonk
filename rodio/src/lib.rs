#![cfg_attr(test, deny(missing_docs))]
use cpal::traits::HostTrait;
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
use decoder::Decoder;
use gonk_types::{Index, Song};
use rand::prelude::SliceRandom;
use rand::thread_rng;

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
use std::time::Duration;

static VOLUME_STEP: u16 = 5;

pub struct Player {
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    pub duration: f64,
    pub volume: u16,
    pub songs: Index<Song>,
}

impl Player {
    pub fn new(volume: u16) -> Self {
        optick::event!("new player");
        let (stream, handle) =
            OutputStream::try_default().expect("Could not create output stream.");
        let sink = Sink::try_new(&handle).unwrap();
        sink.set_volume(f32::from(volume) / 1000.0);

        Self {
            stream,
            handle,
            sink,
            duration: 0.0,
            volume,
            songs: Index::default(),
        }
    }
    pub fn add_songs(&mut self, song: Vec<Song>) {
        self.songs.data.extend(song);
        if self.songs.is_none() && !self.songs.is_empty() {
            self.songs.select(Some(0));
            self.play_selected();
        }
    }
    pub fn play_song(&mut self, i: usize) {
        if self.songs.data.get(i).is_some() {
            self.songs.select(Some(i));
            self.play_selected();
        };
    }
    pub fn clear_songs(&mut self) {
        self.songs = Index::default();
        self.stop();
    }
    pub fn prev_song(&mut self) {
        self.songs.up();
        self.play_selected();
    }
    pub fn next_song(&mut self) {
        self.songs.down();
        self.play_selected();
    }
    pub fn volume_up(&mut self) -> u16 {
        self.volume += VOLUME_STEP;

        if self.volume > 100 {
            self.volume = 100;
        }

        self.update_volume();
        self.volume
    }
    pub fn volume_down(&mut self) -> u16 {
        if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        self.update_volume();
        self.volume
    }
    fn update_volume(&self) {
        if let Some(song) = self.songs.selected() {
            let volume = self.volume as f32 / 1000.0;
            let gain = song.track_gain / 1000.0;
            self.sink.set_volume(volume + gain as f32);
        }
    }
    pub fn play_selected(&mut self) {
        if let Some(song) = self.songs.selected().cloned() {
            self.stop();
            let file = File::open(&song.path).expect("Could not open song.");
            let decoder = Decoder::new(file).unwrap();

            //TODO: wtf is this?
            self.duration = decoder
                .total_duration()
                .expect("could not get duration")
                .as_secs_f64()
                - 0.29;
            self.sink.append(decoder);
            self.update_volume();
        }
    }
    pub fn delete_song(&mut self, selected: usize) {
        //delete the song from the queue
        self.songs.data.remove(selected);

        if let Some(current_song) = self.songs.index {
            let len = self.songs.len();

            if len == 0 {
                self.clear_songs();
                return;
            } else if current_song == selected && selected == 0 {
                self.songs.select(Some(0));
            } else if current_song == selected && len == selected {
                self.songs.select(Some(len - 1));
            } else if selected < current_song {
                self.songs.select(Some(current_song - 1));
            }

            let end = len.saturating_sub(1);

            if selected > end {
                self.songs.select(Some(end));
            }

            //if the playing song was deleted
            //play the next track
            if selected == current_song {
                self.play_selected();
            }
        };
    }
    pub fn randomize(&mut self) {
        if let Some(song) = &self.songs.selected().cloned() {
            self.songs.data.shuffle(&mut thread_rng());

            let mut index = 0;
            for (i, s) in self.songs.data.iter().enumerate() {
                if s == song {
                    index = i;
                }
            }
            self.songs.select(Some(index));
        }
    }
    pub fn stop(&mut self) {
        self.sink = Sink::try_new(&self.handle).expect("Could not create new sink.");
        self.sink.set_volume(f32::from(self.volume) / 1000.0);
    }
    pub fn elapsed(&self) -> Duration {
        self.sink.elapsed()
    }
    pub fn toggle_playback(&self) {
        self.sink.toggle_playback();
    }
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
    pub fn seek_by(&mut self, amount: f64) {
        let mut seek = self.elapsed().as_secs_f64() + amount;
        if seek > self.duration {
            return self.next_song();
        } else if seek < 0.0 {
            seek = 0.0;
        }
        self.sink.seek(Duration::from_secs_f64(seek));
    }
    pub fn seek_to(&self, pos: f64) {
        self.sink.seek(Duration::from_secs_f64(pos));
    }
    pub fn seeker(&self) -> f64 {
        let elapsed = self.elapsed();
        elapsed.as_secs_f64() / self.duration
    }
    pub fn update(&mut self) {
        if self.elapsed().as_secs_f64() > self.duration {
            self.next_song();
        }
    }
    pub fn output_devices() -> Vec<Device> {
        optick::event!("output_devices");
        let host_id = cpal::default_host().id();
        let host = cpal::host_from_id(host_id).unwrap();

        //TODO: this contains inputs aswell as outputs
        //getting just the outputs was too slow 150+ ms
        host.devices().unwrap().collect()
        // devices.sort_by_key(|a| a.name().unwrap().to_lowercase());
        // devices
    }
    pub fn default_device() -> Device {
        cpal::default_host()
            .default_output_device()
            .expect("Could not get default device.")
    }
    pub fn change_output_device(&mut self, device: &Device) -> bool {
        //temp fix so that changing to an input doesn't crash
        if let Ok((stream, handle)) = OutputStream::try_from_device(device) {
            self.stop();
            self.stream = stream;
            self.handle = handle;
            true
        } else {
            false
        }
    }
}

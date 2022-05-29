use cpal::traits::HostTrait;
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
use decoder::Decoder;
use gonk_core::{Index, Song};
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

const VOLUME_STEP: u16 = 5;
const VOLUME_REDUCTION: f32 = 600.0;

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
        let (stream, handle) =
            OutputStream::try_default().expect("Could not create output stream.");
        let sink = Sink::try_new(&handle).unwrap();
        sink.set_volume(f32::from(volume) / VOLUME_REDUCTION);

        Self {
            stream,
            handle,
            sink,
            duration: 0.0,
            volume,
            songs: Index::default(),
        }
    }
    pub fn add_songs(&mut self, song: &[Song]) {
        self.songs.data.extend(song.to_vec());
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
    pub fn clear(&mut self) {
        self.songs = Index::default();
        self.stop();
    }
    //TODO: might remove this?
    pub fn clear_except_playing(&mut self) {
        let selected = self.songs.selected().cloned();
        let mut i = 0;
        while i < self.songs.len() {
            if Some(&self.songs.data[i]) != selected.as_ref() {
                self.songs.data.remove(i);
            } else {
                i += 1;
            }
        }
        self.songs.select(Some(0));
    }
    pub fn prev_song(&mut self) {
        self.songs.up();
        self.play_selected();
    }
    pub fn next_song(&mut self) {
        self.songs.down();
        self.play_selected();
    }
    pub fn volume_up(&mut self) {
        self.volume += VOLUME_STEP;

        if self.volume > 100 {
            self.volume = 100;
        }

        self.update_volume();
    }
    pub fn volume_down(&mut self) {
        if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        self.update_volume();
    }
    fn update_volume(&self) {
        if let Some(song) = self.songs.selected() {
            let volume = self.volume as f32 / VOLUME_REDUCTION;
            let gain = song.track_gain as f32 / VOLUME_REDUCTION;
            self.sink.set_volume(volume + gain);
        }
    }
    pub fn play_selected(&mut self) {
        if let Some(song) = self.songs.selected().cloned() {
            self.stop();
            let file = File::open(&song.path).expect("Could not open song.");
            let decoder = Decoder::new(file).unwrap();

            //FIXME: The duration is slightly off for some reason.
            self.duration = decoder.total_duration().unwrap().as_secs_f64() - 0.29;
            self.sink.append(decoder);
            self.update_volume();
        }
    }
    pub fn delete_song(&mut self, selected: usize) {
        self.songs.data.remove(selected);

        if let Some(playing) = self.songs.index() {
            let len = self.songs.len();

            if len == 0 {
                self.clear();
                return;
            }

            if selected == playing && selected == 0 {
                self.songs.select(Some(0));
            } else if selected == playing && selected == len {
                self.songs.select(Some(len - 1));
            } else if selected < playing {
                self.songs.select(Some(playing - 1));
            }

            if selected == playing {
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
        self.sink
            .set_volume(f32::from(self.volume) / VOLUME_REDUCTION);
    }
    pub fn elapsed(&self) -> f64 {
        self.sink.elapsed().as_secs_f64()
    }
    pub fn toggle_playback(&self) {
        self.sink.toggle_playback();
    }
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
    pub fn seek_by(&mut self, amount: f64) {
        let mut seek = self.elapsed() + amount;
        if seek > self.duration {
            return self.next_song();
        } else if seek < 0.0 {
            seek = 0.0;
        }
        self.sink.seek(Duration::from_secs_f64(seek));
    }
    pub fn seek_to(&self, time: f64) {
        self.sink.seek(Duration::from_secs_f64(time));
        if self.is_paused() {
            self.toggle_playback();
        }
    }
    pub fn update(&mut self) {
        if self.elapsed() > self.duration {
            self.next_song();
        }
    }
    pub fn output_devices() -> Vec<Device> {
        let host_id = cpal::default_host().id();
        let host = cpal::host_from_id(host_id).unwrap();

        //FIXME: Getting just the output devies was too slow(150ms).
        //Instead collect every device available.
        //If this was done lazily the user probably wouldn't notice
        //since the settings menu gets the least amount of use.
        host.devices().unwrap().collect()
    }
    pub fn default_device() -> Option<Device> {
        cpal::default_host().default_output_device()
    }
    //FIXME: This function returns a bool so updating
    //the config can be skipped when selecting an input.
    pub fn change_output_device(&mut self, device: &Device) -> bool {
        //temp fix so that changing to an input doesn't crash
        if let Ok((stream, handle)) = OutputStream::try_from_device(device) {
            let pos = self.elapsed();
            self.stop();
            self.stream = stream;
            self.handle = handle;
            self.play_selected();
            self.seek_to(pos);
            true
        } else {
            false
        }
    }
}

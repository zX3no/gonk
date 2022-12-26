//! TODO: Describe the audio backend
//!
//! Pipewire is currently not implemented because WSL doesn't work well with audio.
//! AND...I don't have a spare drive to put linux on.
#![feature(const_maybe_uninit_zeroed)]
#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]

pub use backend::{default_device, devices, Device};

use backend::Backend;
use decoder::Symphonia;
use gonk_core::{Index, Song};
use std::{path::Path, sync::Once, time::Duration};

mod backend;
pub mod decoder;

#[cfg(windows)]
mod wasapi;

#[cfg(unix)]
mod pipewire;

const VOLUME_REDUCTION: f32 = 150.0;

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        #[cfg(windows)]
        unsafe {
            wasapi::init()
        };
    });
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Stopped,
    Paused,
    Playing,
    Finished,
}

pub struct Player {
    pub songs: Index<Song>,

    backend: Box<dyn Backend>,

    output_device: Device,
    symphonia: Option<Symphonia>,
    sample_rate: usize,
    gain: f32,
    volume: f32,
    elapsed: Duration,
    duration: Duration,
    state: State,
}

impl Player {
    #[allow(clippy::new_without_default)]
    pub fn new(device: &str, volume: u8, songs: Index<Song>, elapsed: f32) -> Self {
        init();

        let devices = devices();
        let default = unsafe { default_device() };
        let device = devices.iter().find(|d| d.name == device);
        let device = device.unwrap_or(default);
        let backend = backend::new(device, None);

        let mut player = Self {
            songs,
            output_device: device.clone(),
            sample_rate: backend.sample_rate(),
            backend,
            symphonia: None,
            gain: 0.5,
            volume: volume as f32 / VOLUME_REDUCTION,
            duration: Duration::default(),
            elapsed: Duration::default(),
            state: State::Stopped,
        };

        //Restore previous queue state.
        if let Some(song) = player.songs.selected().cloned() {
            player.restore_song(song.path.clone(), song.gain, elapsed);
        }

        player
    }

    //TODO: Devices with 4 channels don't play correctly?
    /// Handles all of the player related logic such as:
    ///
    /// - Updating the elapsed time
    /// - Filling the output device with samples.
    /// - Triggering the next song
    pub fn update(&mut self) {
        if self.is_finished() {
            self.next();
        }

        if self.state != State::Playing {
            return;
        }

        //Update the elapsed time and fill the output buffer.
        let Some(symphonia) = &mut self.symphonia else {
            return;
        };

        let gain = if self.gain == 0.0 { 0.5 } else { self.gain };
        self.backend.fill_buffer(self.volume * gain, symphonia);

        if symphonia.is_full() {
            return;
        }

        if let Some(packet) = symphonia.next_packet(&mut self.elapsed, &mut self.state) {
            symphonia.push(packet.samples());
        }
    }
    pub fn update_decoder(&mut self, path: impl AsRef<Path>) {
        match Symphonia::new(path) {
            Ok(sym) => {
                self.duration = sym.duration();
                let new = sym.sample_rate();

                if self.sample_rate != new {
                    self.backend.set_sample_rate(new, &self.output_device);
                    self.sample_rate = new;
                }

                self.symphonia = Some(sym);
            }
            Err(err) => gonk_core::log!("{}", err),
        }
    }
    pub fn play_song(&mut self, path: impl AsRef<Path>, gain: f32) {
        self.state = State::Playing;
        self.elapsed = Duration::default();
        if gain != 0.0 {
            self.gain = gain;
        }
        self.update_decoder(path);
    }
    pub fn restore_song(&mut self, path: impl AsRef<Path>, gain: f32, elapsed: f32) {
        self.state = State::Paused;
        self.elapsed = Duration::from_secs_f32(elapsed);
        if gain != 0.0 {
            self.gain = gain;
        }
        self.update_decoder(path);
        if let Some(decoder) = &mut self.symphonia {
            decoder.seek(elapsed);
        }
    }
    pub fn play(&mut self) {
        self.state = State::Playing;
    }
    pub fn pause(&mut self) {
        self.state = State::Paused;
    }
    pub fn seek(&mut self, pos: f32) {
        if let Some(symphonia) = &mut self.symphonia {
            symphonia.seek(pos);
        }
    }
    pub fn volume_up(&mut self) {
        self.volume =
            ((self.volume * VOLUME_REDUCTION) as u8 + 5).clamp(0, 100) as f32 / VOLUME_REDUCTION;
    }
    pub fn volume_down(&mut self) {
        self.volume =
            ((self.volume * VOLUME_REDUCTION) as i8 - 5).clamp(0, 100) as f32 / VOLUME_REDUCTION;
    }
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn is_playing(&self) -> bool {
        self.state == State::Playing
    }
    pub fn next(&mut self) {
        self.songs.down();
        if let Some(song) = self.songs.selected() {
            self.play_song(song.path.clone(), song.gain);
        }
    }
    pub fn prev(&mut self) {
        self.songs.up();
        if let Some(song) = self.songs.selected() {
            self.play_song(song.path.clone(), song.gain);
        }
    }
    pub fn delete_index(&mut self, index: usize) {
        if self.songs.is_empty() {
            return;
        }

        self.songs.remove(index);

        if let Some(playing) = self.songs.index() {
            let len = self.songs.len();
            if len == 0 {
                self.clear();
            } else if index == playing && index == 0 {
                self.songs.select(Some(0));
                self.play_index(self.songs.index().unwrap());
            } else if index == playing && index == len {
                self.songs.select(Some(len - 1));
                self.play_index(self.songs.index().unwrap());
            } else if index < playing {
                self.songs.select(Some(playing - 1));
            }
        };
    }
    pub fn clear(&mut self) {
        self.state = State::Stopped;
        self.symphonia = None;
        self.songs = Index::default();
    }
    pub fn clear_except_playing(&mut self) {
        if let Some(index) = self.songs.index() {
            let playing = self.songs.remove(index);
            self.songs = Index::new(vec![playing], Some(0));
        }
    }
    pub fn add(&mut self, songs: Vec<Song>) {
        self.songs.extend(songs);
        if self.songs.selected().is_none() {
            self.songs.select(Some(0));
            self.play_index(0);
        }
    }
    pub fn play_index(&mut self, i: usize) {
        self.songs.select(Some(i));
        if let Some(song) = self.songs.selected() {
            self.play_song(song.path.clone(), song.gain);
        }
    }
    pub fn toggle_playback(&mut self) {
        match self.state {
            State::Paused => self.play(),
            State::Playing => self.pause(),
            _ => (),
        }
    }
    pub fn is_finished(&self) -> bool {
        self.state == State::Finished
    }
    pub fn seek_foward(&mut self) {
        let pos = (self.elapsed().as_secs_f32() + 10.0).clamp(0.0, f32::MAX);
        self.seek(pos);
    }
    pub fn seek_backward(&mut self) {
        let pos = (self.elapsed().as_secs_f32() - 10.0).clamp(0.0, f32::MAX);
        self.seek(pos);
    }
    pub fn volume(&self) -> u8 {
        (self.volume * VOLUME_REDUCTION) as u8
    }
    pub fn set_output_device(&mut self, device: &str) {
        let devices = devices();
        let device = if let Some(device) = devices.iter().find(|d| d.name == device) {
            device
        } else {
            unreachable!("Requested a device that does not exist.")
        };

        self.backend = backend::new(device, Some(self.sample_rate));
    }
}

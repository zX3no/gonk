pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
use gonk_types::{Index, Song};
use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::{
        static_sound::PlaybackState,
        streaming::{StreamingSoundData, StreamingSoundSettings},
    },
    sound::{streaming::StreamingSoundHandle, FromFileError},
    tween::Tween,
    Volume,
};
use rand::prelude::SliceRandom;
use rand::thread_rng;

static VOLUME_STEP: u16 = 5;

pub struct Player {
    manager: AudioManager<CpalBackend>,
    handle: Option<StreamingSoundHandle<FromFileError>>,
    pub volume: u16,
    pub songs: Index<Song>,
}

impl Player {
    pub fn new(volume: u16) -> Self {
        Self {
            manager: AudioManager::<CpalBackend>::new(AudioManagerSettings::default()).unwrap(),
            handle: None,
            volume,
            songs: Index::default(),
        }
    }
    pub fn play(&mut self) {
        if !self.songs.is_empty() {
            self.stop();
        }

        if let Some(song) = &self.songs.selected() {
            let v = f64::from(self.volume) / 1000.0;

            let sound = StreamingSoundData::from_file(
                &song.path,
                StreamingSoundSettings::new().volume(Volume::Amplitude(v)),
            )
            .unwrap();

            self.handle = Some(self.manager.play(sound).unwrap());
        }
    }
    pub fn play_index(&mut self, i: usize) {
        self.songs.select(Some(i));
        self.play();
    }
    pub fn add_songs(&mut self, song: &[Song]) {
        //TODO: remove to vec
        self.songs.data.extend(song.to_vec());
        if !self.songs.is_empty() {
            self.songs.select(Some(0));
            self.play();
        }
    }
    pub fn stop(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.stop(Tween::default()).unwrap();
        }
    }
    pub fn clear_songs(&mut self) {
        self.songs = Index::default();
        self.stop();
    }
    pub fn prev_song(&mut self) {
        self.songs.down();
        self.play();
    }
    pub fn next_song(&mut self) {
        self.songs.up();
        self.play();
    }
    pub fn volume_up(&mut self) -> u16 {
        self.volume += VOLUME_STEP;

        if self.volume > 100 {
            self.volume = 100;
        }

        if let Some(handle) = &mut self.handle {
            let v = f64::from(self.volume) / 1000.0;
            handle
                .set_volume(Volume::Amplitude(v), Tween::default())
                .unwrap();
        }

        self.volume
    }
    pub fn volume_down(&mut self) -> u16 {
        if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        if let Some(handle) = &mut self.handle {
            let v = f64::from(self.volume) / 1000.0;
            handle
                .set_volume(Volume::Amplitude(v), Tween::default())
                .unwrap();
        }

        self.volume
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
                self.play();
            }
        };
    }
    pub fn randomize(&mut self) {
        if !self.songs.is_empty() {
            self.songs.data.shuffle(&mut thread_rng());

            if let Some(song) = &self.songs.selected() {
                let mut index = 0;
                for (i, s) in self.songs.data.iter().enumerate() {
                    if &s == song {
                        index = i;
                    }
                }
                self.songs.select(Some(index));
            }
        }
    }
    pub fn elapsed(&self) -> Option<f64> {
        self.handle.as_ref().map(|handle| handle.position())
    }
    pub fn duration(&self) -> Option<f64> {
        self.songs
            .selected()
            .map(|song| song.duration.as_secs_f64())
    }
    pub fn toggle_playback(&mut self) {
        if let Some(handle) = &mut self.handle {
            match handle.state() {
                PlaybackState::Playing | PlaybackState::Pausing => {
                    handle.resume(Tween::default()).unwrap()
                }
                PlaybackState::Paused => handle.pause(Tween::default()).unwrap(),
                _ => (),
            };
        }
    }
    pub fn is_playing(&self) -> bool {
        if let Some(handle) = &self.handle {
            match handle.state() {
                PlaybackState::Playing => true,
                PlaybackState::Pausing => false,
                PlaybackState::Paused => false,
                _ => false,
            }
        } else {
            false
        }
    }
    pub fn seek_fw(&mut self) {
        if let Some(elapsed) = self.elapsed() {
            let seek = elapsed + 10.0;
            if let Some(duration) = self.duration() {
                if seek > duration {
                    self.next_song();
                } else {
                    self.seek_to(seek);
                }
            }
        }
    }
    pub fn seek_bw(&mut self) {
        if let Some(elapsed) = self.elapsed() {
            let mut seek = elapsed - 10.0;

            if seek < 0.0 {
                seek = 0.0;
            }

            self.seek_to(seek);
        }
    }
    pub fn seek_to(&mut self, time: f64) {
        if let Some(handle) = &mut self.handle {
            handle.seek_to(time).unwrap();
        }
    }
    pub fn seeker(&self) -> f64 {
        if let Some(duration) = self.duration() {
            if let Some(elapsed) = self.elapsed() {
                return elapsed / duration;
            }
        }
        0.0
    }
    pub fn update(&mut self) {
        //TODO: check the state of the player and then skip if it's stopped.
        if let Some(duration) = self.duration() {
            if let Some(elapsed) = self.elapsed() {
                if elapsed > duration {
                    self.next_song();
                }
            }
        }
    }
    // pub fn output_devices() -> Vec<Device> {
    //     let host_id = cpal::default_host().id();
    //     let host = cpal::host_from_id(host_id).unwrap();

    //     let mut devices: Vec<_> = host.output_devices().unwrap().collect();
    //     devices.sort_by_key(|a| a.name().unwrap().to_lowercase());
    //     devices
    // }
    // pub fn default_device() -> Option<Device> {
    //     cpal::default_host().default_output_device()
    // }
    // pub fn change_output_device(&mut self, device: &Device) {
    //     self.stop();
    //     let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings {
    //         ..Default::default()
    //     })
    //     .unwrap();
    // }
}

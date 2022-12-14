#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case,
    clippy::type_complexity
)]
use crossbeam_channel::{bounded, Receiver, Sender};
use decoder::{Symphonia, BUFFER};
use gonk_core::{Index, Song};
use std::{sync::Once, thread, time::Duration};

pub mod decoder;

#[cfg(windows)]
mod wasapi;

#[cfg(windows)]
pub use wasapi::*;

#[cfg(unix)]
mod pipewire;

#[cfg(unix)]
pub use pipewire::*;

const VOLUME_REDUCTION: f32 = 150.0;

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| unsafe {
        #[cfg(windows)]
        wasapi::init();
    });
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Stopped,
    Paused,
    Playing,
    Finished,
}

#[derive(Debug)]
pub enum Event {
    /// Path, Gain
    PlaySong((String, f32)),
    OutputDevice(String),
    Play,
    Pause,
    Stop,
    Seek(f32),
}

//TODO: Devices with 4 channels don't play correctly?
pub unsafe fn new(device: &Device, r: Receiver<Event>) {
    let mut wasapi = Wasapi::new(device, None);
    let mut sample_rate = wasapi.format.Format.nSamplesPerSec as usize;
    let mut gain = 0.50;
    let mut symphonia = None;

    loop {
        if let Ok(event) = r.try_recv() {
            match event {
                Event::PlaySong((path, g)) => {
                    STATE = State::Playing;
                    ELAPSED = Duration::default();
                    if g != 0.0 {
                        gain = g;
                    }
                    match Symphonia::new(path) {
                        Ok(d) => {
                            DURATION = d.duration();

                            let new = d.sample_rate();
                            if sample_rate != new {
                                if wasapi.set_sample_rate(new).is_err() {
                                    wasapi = Wasapi::new(device, Some(new));
                                };
                                sample_rate = new;
                            }

                            symphonia = Some(d);
                        }
                        Err(err) => gonk_core::log!("{}", err),
                    }
                }
                Event::Seek(pos) => {
                    if let Some(symphonia) = &mut symphonia {
                        symphonia.seek(pos);
                    }
                }
                Event::Play => STATE = State::Playing,
                Event::Pause => STATE = State::Paused,
                Event::Stop => {
                    STATE = State::Stopped;
                    symphonia = None;
                }
                Event::OutputDevice(device) => {
                    let device = if let Some(device) = devices().iter().find(|d| d.name == device) {
                        device
                    } else {
                        unreachable!("Requested a device that does not exist.")
                    };
                    wasapi = Wasapi::new(device, Some(sample_rate));
                }
            }
        }

        //HACK: How to make blocking?
        thread::sleep(Duration::from_millis(2));

        if State::Playing != STATE {
            continue;
        }

        //Update the elapsed time and fill the output buffer.
        let Some(symphonia) = &mut symphonia else {
            continue;
        };

        ELAPSED = symphonia.elapsed();
        wasapi.fill_buffer(gain, symphonia);

        if BUFFER.is_full() {
            continue;
        }

        if let Some(packet) = symphonia.next_packet() {
            BUFFER.push(packet.samples());
        }
    }
}

pub struct Player {
    s: Sender<Event>,
    pub songs: Index<Song>,
}

impl Player {
    #[allow(clippy::new_without_default)]
    pub fn new(device: &str, volume: u8, songs: Index<Song>, elapsed: f32) -> Self {
        init();

        let devices = devices();
        let default = default_device().unwrap();
        let d = devices.iter().find(|d| d.name == device);
        let device = if let Some(d) = d { d } else { default };

        let (s, r) = bounded::<Event>(5);
        thread::spawn(move || unsafe {
            new(device, r);
        });

        //Restore previous queue state.
        unsafe { VOLUME = volume as f32 / VOLUME_REDUCTION };
        if let Some(song) = songs.selected().cloned() {
            s.send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
            s.send(Event::Seek(elapsed)).unwrap();
        }

        Self { s, songs }
    }
    pub fn play(&self) {
        self.s.send(Event::Play).unwrap();
    }
    pub fn pause(&self) {
        self.s.send(Event::Pause).unwrap();
    }
    pub fn seek(&self, pos: f32) {
        self.s.send(Event::Seek(pos)).unwrap();
    }
    pub fn volume_up(&self) {
        unsafe {
            VOLUME =
                ((VOLUME * VOLUME_REDUCTION) as u8 + 5).clamp(0, 100) as f32 / VOLUME_REDUCTION;
        }
    }
    pub fn volume_down(&self) {
        unsafe {
            VOLUME =
                ((VOLUME * VOLUME_REDUCTION) as i8 - 5).clamp(0, 100) as f32 / VOLUME_REDUCTION;
        }
    }
    pub fn elapsed(&self) -> Duration {
        unsafe { ELAPSED }
    }
    pub fn duration(&self) -> Duration {
        unsafe { DURATION }
    }
    pub fn is_playing(&self) -> bool {
        unsafe { STATE == State::Playing }
    }
    pub fn next(&mut self) {
        self.songs.down();
        if let Some(song) = self.songs.selected() {
            unsafe { STATE == State::Playing };
            self.s
                .send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
        }
    }
    pub fn prev(&mut self) {
        self.songs.up();
        if let Some(song) = self.songs.selected() {
            self.s
                .send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
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
        self.s.send(Event::Stop).unwrap();
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
            self.s
                .send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
        }
    }
    pub fn toggle_playback(&self) {
        match unsafe { &STATE } {
            State::Paused => self.play(),
            State::Playing => self.pause(),
            _ => (),
        }
    }
    pub fn is_finished(&self) -> bool {
        unsafe { STATE == State::Finished }
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
        unsafe { (VOLUME * VOLUME_REDUCTION) as u8 }
    }
    pub fn set_output_device(&self, device: &str) {
        self.s
            .send(Event::OutputDevice(device.to_string()))
            .unwrap();
    }
}

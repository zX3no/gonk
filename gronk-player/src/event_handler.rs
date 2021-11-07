#![allow(dead_code)]
use soloud::{AudioExt, Handle, LoadExt, Soloud, Wav};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Event {
    Volume(f32),
    TogglePlayback,
    Play(PathBuf),
    Stop,
    Null,
}

pub struct EventHandler {
    volume: f32,
    ctx: Soloud,
    handle: Option<Handle>,
    wav: Option<Wav>,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            volume: 0.03,
            ctx: Soloud::default().unwrap(),
            handle: None,
            wav: None,
        }
    }
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Volume(v) => self.set_volume(v),
            Event::TogglePlayback => self.toggle_playback(),
            Event::Play(song) => self.play_file(&song),
            Event::Stop => self.stop(),
            Event::Null => (),
        }
    }
    pub fn play_file(&mut self, path: &Path) {
        self.stop();
        self.set_wav(path);

        if let Some(wav) = &self.wav {
            self.handle = Some(self.ctx.play(wav))
        } else {
            panic!();
        }
    }
    pub fn set_wav(&mut self, path: &Path) {
        let mut wav = Wav::default();
        let bytes = std::fs::read(path).unwrap();
        wav.load_mem(&bytes).unwrap();
        self.wav = Some(wav);
    }
    pub fn toggle_playback(&mut self) {
        if let Some(handle) = self.handle {
            let paused = self.ctx.pause(handle);
            if paused {
                self.ctx.set_pause_all(false);
            } else {
                self.ctx.set_pause_all(true);
            }
        }
    }
    pub fn stop(&mut self) {
        self.ctx.stop_all();
    }
    pub fn get_elapsed(&self) -> f64 {
        if let Some(handle) = self.handle {
            return self.ctx.stream_position(handle);
        }
        0.0
    }
    pub fn get_duration(&mut self) -> f64 {
        if let Some(wav) = &self.wav {
            return wav.length();
        }
        0.0
    }
    pub fn is_playing(&self) -> bool {
        if let Some(handle) = self.handle {
            return self.ctx.is_valid_voice_handle(handle);
        }
        false
    }
    pub fn set_volume(&mut self, v: f32) {
        self.volume += v;
        if self.volume < 0.0 {
            self.volume = 0.0;
        } else if self.volume > 1.0 {
            self.volume = 1.0;
        }
        self.ctx.set_global_volume(self.volume);
    }
    pub fn fix_volume(&mut self) {
        self.ctx.set_global_volume(self.volume);
    }
}

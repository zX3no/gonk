#![allow(dead_code)]
use soloud::{AudioExt, Handle, LoadExt, Soloud, Wav};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Event {
    Volume(f32),
    TogglePlayback,
    Seek(f64),
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
            volume: 0.02,
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
            Event::Seek(amount) => self.seek(amount),
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
    pub fn is_playing(&self) -> bool {
        if let Some(handle) = self.handle {
            return self.ctx.is_valid_voice_handle(handle);
        }
        false
    }
    pub fn seeker(&self) -> f64 {
        let duration = if let Some(wav) = &self.wav {
            wav.length()
        } else {
            0.0
        };
        let elapsed = if let Some(handle) = self.handle {
            self.ctx.stream_position(handle)
        } else {
            0.0
        };

        elapsed / duration
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
    fn seek(&mut self, secs: f64) {
        if let Some(handle) = self.handle {
            if let Some(wav) = &self.wav {
                let elapsed = self.ctx.stream_position(handle);
                let length = wav.length();
                let new_pos = elapsed + secs;

                if new_pos < length && new_pos > 0.0 {
                    self.ctx.seek(handle, new_pos).unwrap();
                    eprintln!(
                        "elapsed: {}, secs: {}, new_pos: {}, length: {}",
                        elapsed, secs, new_pos, length,
                    );
                } else if new_pos > length {
                    eprintln!("stop");
                    self.stop();
                } else {
                    eprintln!("start");
                    self.ctx.seek(handle, 0.0).unwrap();
                }
            }
        }
    }
}

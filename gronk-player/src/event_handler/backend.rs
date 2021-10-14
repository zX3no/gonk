use std::{
    path::PathBuf,
    sync::{mpsc::channel, Arc, Mutex, MutexGuard},
    thread::JoinHandle,
};

use soloud::*;

pub struct Backend {
    ctx: Soloud,
    //allows access to currently playing song
    handle: Option<Handle>,
    wav: Option<Wav>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            ctx: Soloud::default().unwrap(),
            handle: None,
            wav: None,
        }
    }
    pub fn play_file(&mut self, path: &PathBuf) {
        self.stop();
        self.set_wav(path);

        if let Some(wav) = &self.wav {
            self.handle = Some(self.ctx.play(wav))
        } else {
            panic!();
        }
    }

    pub fn set_wav(&mut self, path: &PathBuf) {
        let mut wav = audio::Wav::default();
        wav.load(path).unwrap();
        self.wav = Some(wav);
    }

    pub fn play(&mut self) {
        self.ctx.set_pause_all(false);
    }
    pub fn pause(&mut self) {
        self.ctx.set_pause_all(true);
    }
    pub fn stop(&mut self) {
        self.ctx.stop_all();
    }
    pub fn set_volume(&mut self, v: f32) {
        self.ctx.set_global_volume(v);
    }
    //these shouldn't be here the next track should be set by the queue
    pub fn next(&mut self) {
        todo!();
    }
    pub fn previous(&mut self) {
        todo!();
    }
    pub fn get_elapsed(&self) -> f64 {
        if let Some(handle) = self.handle {
            return self.ctx.stream_position(handle);
        }
        0.0
    }
}

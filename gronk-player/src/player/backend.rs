use std::path::Path;

use soloud::*;

pub struct Backend {
    ctx: Soloud,
    //allows access to currently playing song
    handle: Option<Handle>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            ctx: Soloud::default().unwrap(),
            handle: None,
        }
    }
    // pub fn play(&mut self, path: &Path) {
    //     let mut wav = audio::Wav::default();
    //     wav.load(path).unwrap();
    //     //update the current song handle
    //     //and play the song
    //     self.handle = Some(self.ctx.play(&wav));
    // }
    pub fn play_file(&mut self, path: &Path) {
        //stop playback
        self.stop();

        //load wav file
        let mut wav = audio::Wav::default();
        wav.load(path).unwrap();

        //update the current song handle
        //and play the song
        self.handle = Some(self.ctx.play(&wav));
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

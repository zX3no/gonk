use std::{
    path::PathBuf,
    sync::{mpsc::channel, Arc, Mutex, MutexGuard},
    thread::{self, JoinHandle},
};

use soloud::*;

pub struct Backend {
    ctx: Arc<Mutex<Soloud>>,
    //allows access to currently playing song
    handle: Option<Handle>,
    join_handle: Option<JoinHandle<()>>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(Mutex::new(Soloud::default().unwrap())),
            handle: None,
            join_handle: None,
        }
    }
    pub fn play_file(&mut self, path: &PathBuf) {
        self.stop();

        let path = path.clone();
        let ctx = self.ctx.clone();
        println!("{:?}", path);

        let (tx, rx) = channel();
        self.join_handle = Some(thread::spawn(move || {
            let mut wav = audio::Wav::default();
            wav.load(path).unwrap();

            let handle = Some(ctx.lock().unwrap().play(&wav));
            tx.send(handle).unwrap();
            thread::park();
        }));
        self.handle = rx.recv().unwrap();
    }
    pub fn ctx(&self) -> MutexGuard<'_, soloud::Soloud> {
        self.ctx.lock().unwrap()
    }
    pub fn play(&mut self) {
        self.ctx().set_pause_all(false);
    }
    pub fn pause(&mut self) {
        self.ctx().set_pause_all(true);
    }
    pub fn stop(&mut self) {
        self.ctx().stop_all();

        //remove the thread
        if let Some(handle) = &self.join_handle {
            handle.thread().unpark();
        }
    }
    pub fn set_volume(&mut self, v: f32) {
        self.ctx().set_global_volume(v);
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
            return self.ctx().stream_position(handle);
        }
        0.0
    }
}

use std::{
    path::PathBuf,
    sync::{
        mpsc::{channel, Sender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use crate::event_handler::{Event, EventHandler};
mod event_handler;

#[derive(Debug)]
pub struct Player {
    tx: Sender<Event>,
    playing: Arc<RwLock<bool>>,
    seeker: Arc<RwLock<f64>>,
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        let playing = Arc::new(RwLock::new(false));
        let seeker = Arc::new(RwLock::new(0.0));
        let p = playing.clone();
        let s = seeker.clone();

        thread::spawn(move || {
            let mut handler = EventHandler::new();
            handler.fix_volume();
            loop {
                let events = rx.try_recv().unwrap_or(Event::Null);
                handler.update(events);

                *p.write().unwrap() = handler.is_playing();
                *s.write().unwrap() = handler.seeker();

                thread::sleep(Duration::from_millis(10));
            }
        });

        Self {
            tx,
            playing,
            seeker,
        }
    }
    pub fn play(&self, song: PathBuf) {
        self.tx.send(Event::Play(song)).unwrap();
    }
    pub fn stop(&self) {
        self.tx.send(Event::Stop).unwrap();
    }
    pub fn volume(&self, v: f32) {
        self.tx.send(Event::Volume(v)).unwrap();
    }
    pub fn toggle_playback(&self) {
        self.tx.send(Event::TogglePlayback).unwrap();
    }
    pub fn is_playing(&self) -> bool {
        *self.playing.read().unwrap()
    }
    pub fn seeker(&self) -> f64 {
        *self.seeker.read().unwrap()
    }
    pub fn seek(&self, amount: i32) {
        self.tx.send(Event::Seek(amount as f64)).unwrap();
    }
}

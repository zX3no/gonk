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
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        let playing = Arc::new(RwLock::new(false));
        let p = playing.clone();

        thread::spawn(move || {
            let mut handler = EventHandler::new();
            loop {
                let events = rx.try_recv().unwrap_or(Event::Null);
                handler.update(events);

                *p.write().unwrap() = handler.is_playing();

                thread::sleep(Duration::from_millis(10));
            }
        });

        Self { tx, playing }
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
}

use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc, RwLock,
    },
    thread,
};

use crate::event_handler::{Event, EventHandler};
use gronk_indexer::database::Song;

#[derive(Debug)]
pub struct Player {
    tx: Sender<Event>,
    seeker: Arc<RwLock<String>>,
    queue: Arc<RwLock<Vec<String>>>,
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let seeker = Arc::new(RwLock::new(String::from("00:00")));
        let queue = Arc::new(RwLock::new(Vec::new()));

        let s = seeker.clone();
        let q = queue.clone();

        thread::spawn(move || {
            let mut h = EventHandler::new();
            let mut events = Event::Null;
            loop {
                h.update(events);
                *s.write().unwrap() = h.get_seeker();
                *q.write().unwrap() = h.get_queue();
                events = rx.recv().unwrap();
            }
        });
        Self { tx, seeker, queue }
    }
    pub fn next(&self) {
        self.tx.send(Event::Next).unwrap();
    }
    pub fn previous(&self) {
        self.tx.send(Event::Previous).unwrap();
    }
    pub fn stop(&self) {
        self.tx.send(Event::Stop).unwrap();
    }
    pub fn volume(&self, v: f32) {
        self.tx.send(Event::Volume(v)).unwrap();
    }
    pub fn add(&self, song: Song) {
        self.tx.send(Event::Add(song)).unwrap();
    }
    pub fn remove(&self, song: Song) {
        self.tx.send(Event::Remove(song)).unwrap();
    }
    pub fn clear_queue(&self) {
        self.tx.send(Event::ClearQueue).unwrap();
    }
    pub fn get_seeker(&self) -> String {
        let seeker = self.seeker.clone();
        let s = seeker.read().unwrap().clone();
        s
    }
    pub fn get_queue(&self) -> Vec<String> {
        let queue = self.queue.clone();
        let q = queue.read().unwrap().clone();
        q
    }
    pub fn toggle_playback(&self) {
        self.tx.send(Event::TogglePlayback).unwrap();
    }
}

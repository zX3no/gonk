use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use gronk_types::Song;

use crate::{
    event_handler::{Event, EventHandler},
    queue::Queue,
};

#[derive(Debug)]
pub struct Player {
    tx: Sender<Event>,
    seeker: Arc<RwLock<String>>,
    queue: Arc<RwLock<Queue>>,
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let seeker = Arc::new(RwLock::new(String::from("00:00")));
        let queue = Arc::new(RwLock::new(Queue::new()));

        let s = seeker.clone();
        let q = queue.clone();

        thread::spawn(move || {
            let mut h = EventHandler::new();
            let mut events;
            loop {
                *s.write().unwrap() = h.get_seeker();
                *q.write().unwrap() = h.get_queue();
                events = rx.try_recv().unwrap_or(Event::Null);
                h.update(events);
                thread::sleep(Duration::from_millis(10));
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
    pub fn add(&self, songs: Vec<Song>) {
        self.tx.send(Event::Add(songs)).unwrap();
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
    pub fn get_queue(&self) -> Queue {
        let queue = self.queue.clone();
        let q = queue.read().unwrap().clone();
        q
    }
    pub fn toggle_playback(&self) {
        self.tx.send(Event::TogglePlayback).unwrap();
    }
}

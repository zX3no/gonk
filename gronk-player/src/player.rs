use std::{
    sync::mpsc::{channel, Sender},
    thread,
};

use crate::event_handler::{Event, EventHandler};

pub struct Player {
    tx: Sender<Event>,
}
impl Player {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut h = EventHandler::new();
            let mut events = Event::Null;
            loop {
                h.update(events);
                events = rx.recv().unwrap();
            }
        });
        Self { tx }
    }
    pub fn next(&self) {
        self.tx.send(Event::Next).unwrap();
    }
    pub fn previous(&mut self) {
        self.tx.send(Event::Previous).unwrap();
    }
    pub fn play(&mut self) {
        self.tx.send(Event::Play).unwrap();
    }
    pub fn pause(&mut self) {
        self.tx.send(Event::Pause).unwrap();
    }
    pub fn stop(&mut self) {
        self.tx.send(Event::Stop).unwrap();
    }
    pub fn volume(&mut self, v: f32) {
        self.tx.send(Event::Volume(v)).unwrap();
    }
}

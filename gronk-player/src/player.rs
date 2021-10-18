use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    thread,
};

use crate::event_handler::{song::Song, Event, EventHandler};

#[derive(Debug)]
pub struct Seeker {
    pub elapsed: String,
    pub duration: String,
}
impl Seeker {
    pub fn new() -> Self {
        Self {
            elapsed: String::new(),
            duration: String::new(),
        }
    }
}

pub struct Player {
    tx: Sender<Event>,
    seeker: Arc<Mutex<Seeker>>,
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let seeker = Arc::new(Mutex::new(Seeker::new()));

        let s = seeker.clone();
        thread::spawn(move || {
            let mut h = EventHandler::new();
            let mut events = Event::Null;
            loop {
                h.update(events);
                *s.lock().unwrap() = h.get_seeker();
                events = rx.recv().unwrap();
            }
        });
        Self { tx, seeker }
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
    pub fn add(&mut self, song: Song) {
        self.tx.send(Event::Add(song)).unwrap();
    }
    pub fn remove(&mut self, song: Song) {
        self.tx.send(Event::Remove(song)).unwrap();
    }
    pub fn clear_queue(&mut self) {
        self.tx.send(Event::ClearQueue).unwrap();
    }
    pub fn get_seeker(&self) -> Seeker {
        let seeker = self.seeker.clone();
        let s = Arc::try_unwrap(seeker).unwrap();
        s.into_inner().unwrap()
    }
}

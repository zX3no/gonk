use std::path::Path;

use backend::Backend;
use song::Song;

mod backend;
mod song;

pub enum Event {
    Next,
    Previous,
    Volume(f32),
    Play,
    Pause,
    Stop,
    Null,
}

pub struct Player {
    queue: Vec<Song>,
    now_playing: Option<Song>,
    volume: f32,
    elapsed: Option<f32>,
    duration: Option<f32>,
    backend: Backend,
    events: Event,
}

impl Player {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            now_playing: None,
            volume: 1.0,
            elapsed: None,
            duration: None,
            backend: Backend::new(),
            events: Event::Null,
        }
    }
    pub fn run(self) {
        let Self {
            queue,
            now_playing,
            volume,
            elapsed,
            duration,
            mut backend,
            mut events,
        } = self;

        loop {
            match events {
                Event::Next => backend.next(),
                Event::Previous => backend.previous(),
                Event::Volume(v) => backend.volume(v),
                Event::Play => backend.play(),
                Event::Pause => backend.pause(),
                Event::Stop => backend.stop(),
                Event::Null => (),
            }
            if let Some(elapsed) = elapsed {
                if elapsed == 0.0 {
                    events = Event::Next;
                }
            }
        }
    }
    pub fn next(&mut self) {
        self.events = Event::Next;
    }
    pub fn previous(&mut self) {
        self.events = Event::Previous;
    }
    pub fn play(&mut self) {
        self.events = Event::Play;
    }
    pub fn pause(&mut self) {
        self.events = Event::Pause;
    }
    pub fn stop(&mut self) {
        self.events = Event::Stop;
    }
    pub fn volume(&mut self, v: f32) {
        self.events = Event::Volume(v);
    }
}

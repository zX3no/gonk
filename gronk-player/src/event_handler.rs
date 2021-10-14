use std::{path::Path, thread, time::Duration};

use backend::Backend;
use song::Song;

mod backend;
mod song;

#[derive(Debug)]
pub enum Event {
    Next,
    Previous,
    Volume(f32),
    Play,
    Pause,
    Stop,
    Null,
}

pub struct EventHandler {
    queue: Vec<Song>,
    now_playing: Option<Song>,
    index: Option<usize>,
    volume: f32,
    backend: Backend,
}

impl EventHandler {
    pub fn new() -> Self {
        let queue = vec![
            Song::from("1.flac"),
            Song::from("2.flac"),
            Song::from("3.flac"),
        ];

        Self {
            queue,
            now_playing: None,
            index: None,
            volume: 1.0,
            backend: Backend::new(),
        }
    }
    pub fn update(&mut self, events: Event) {
        let Self {
            queue,
            now_playing,
            index,
            volume: _,
            backend,
        } = self;

        dbg!(&events);

        match events {
            Event::Next => {
                if let Some(song) = now_playing {
                    if let Some(index) = index {
                        if let Some(next_song) = queue.get(*index + 1) {
                            *song = next_song.clone();
                            *index = *index + 1;
                        } else if let Some(next_song) = queue.first() {
                            *song = next_song.clone();
                            *index = 0;
                        }
                    }
                    backend.play_file(song.get_path());
                }
            }
            Event::Previous => {
                if let Some(song) = now_playing {
                    if let Some(index) = index {
                        if let Some(next_song) = queue.get(*index - 1) {
                            *song = next_song.clone();
                            *index = *index - 1;
                        } else if let Some(next_song) = queue.first() {
                            *song = next_song.clone();
                            *index = 0;
                        }
                    }
                    backend.play_file(song.get_path());
                }
            }
            Event::Volume(v) => backend.set_volume(v),
            Event::Play => backend.play(),
            Event::Pause => backend.pause(),
            Event::Stop => backend.stop(),
            Event::Null => (),
        }

        //check if anything is playing
        if let Some(now_playing) = now_playing {
            //update the time elapsed
            now_playing.update_elapsed(backend.get_elapsed());

            //check if the song has finished
            if let Some(elapsed) = now_playing.get_elapsed() {
                if elapsed == 0.0 {
                    // *events = Event::Next;
                }
            }
        } else {
            //add the first song to the queue
            if let Some(song) = queue.first() {
                println!("Playing..");
                *now_playing = Some(song.clone());
                *index = Some(0);
                backend.set_wav(song.get_path());
                backend.play_file(song.get_path());
                // thread::park();
            } else {
                //Nothing to do...
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
    // pub fn next(&mut self) {
    //     self.events = Event::Next;
    // }
    // pub fn previous(&mut self) {
    //     self.events = Event::Previous;
    // }
    // pub fn play(&mut self) {
    //     self.events = Event::Play;
    // }
    // pub fn pause(&mut self) {
    //     self.events = Event::Pause;
    // }
    // pub fn stop(&mut self) {
    //     self.events = Event::Stop;
    // }
    // pub fn volume(&mut self, v: f32) {
    //     //this needs to be done differently
    //     self.events = Event::Volume(v);
    // }
    pub fn get_elapsed(&self) -> String {
        if let Some(song) = &self.now_playing {
            if let Some(elapsed) = song.get_elapsed() {
                return elapsed.to_string();
            }
        }
        String::from("00:00")
    }
    pub fn get_duration(&self) -> String {
        if let Some(song) = &self.now_playing {
            return song.get_duration().to_string();
        }
        String::from("00:00")
    }
    pub fn get_volume(&self) -> String {
        self.volume.to_string()
    }
}

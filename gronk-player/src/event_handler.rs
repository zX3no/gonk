use std::{thread, time::Duration};

use backend::Backend;
use song::Song;

pub mod backend;
pub mod song;

#[derive(Debug)]
pub enum Event {
    Add(Song),
    Remove(Song),
    ClearQueue,
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
        let queue = vec![Song::from("music/覆.flac")];
        let mut backend = Backend::new();
        let volume = 0.02;
        backend.set_volume(volume);

        Self {
            queue,
            now_playing: None,
            index: None,
            volume,
            backend,
        }
    }
    fn handle_events(&mut self, event: Event) {
        println!("{:#?}", event);
        match event {
            Event::Add(song) => self.add(song),
            Event::ClearQueue => self.clear_queue(),
            Event::Remove(song) => self.remove(song),
            Event::Next => self.next(),
            Event::Previous => self.previous(),
            Event::Volume(v) => self.backend.set_volume(v),
            Event::Play => self.backend.play(),
            Event::Pause => self.backend.pause(),
            Event::Stop => self.backend.stop(),
            Event::Null => (),
        }
    }
    pub fn update(&mut self, event: Event) {
        self.handle_events(event);

        let Self {
            queue,
            now_playing,
            index,
            volume: _,
            backend,
        } = self;

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
                *now_playing = Some(song.clone());
                *index = Some(0);
                backend.play_file(song.get_path());
            } else {
                //Nothing to do...
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
    fn next(&mut self) {
        self.change_song(true);
    }
    fn previous(&mut self) {
        self.change_song(false);
    }
    fn change_song(&mut self, dir: bool) {
        let (now_playing, index, queue) = (&mut self.now_playing, &mut self.index, &self.queue);

        if let Some(song) = now_playing {
            if let Some(index) = index {
                if let Some(next_song) = queue.get(*index + dir as usize) {
                    *song = next_song.clone();
                    *index = *index + dir as usize;
                } else if let Some(next_song) = queue.first() {
                    *song = next_song.clone();
                    *index = 0;
                }
            }
            let song = song.clone();
            self.backend.play_file(song.get_path());
        }
    }
    pub fn add(&mut self, song: Song) {
        self.queue.push(song);
    }
    pub fn remove(&mut self, song: Song) {
        let queue = self.queue.clone();
        for (i, s) in queue.iter().enumerate() {
            if s == &song {
                self.queue.remove(i);
            }
        }
    }
    pub fn clear_queue(&mut self) {
        self.queue.drain(..);
    }
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

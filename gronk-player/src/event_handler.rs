use std::{thread, time::Duration};

use backend::Backend;
use gronk_indexer::database::Song;

pub mod backend;

#[derive(Debug)]
pub enum Event {
    Add(Vec<Song>),
    Remove(Song),
    ClearQueue,
    Next,
    Previous,
    Volume(f32),
    TogglePlayback,
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
        let mut backend = Backend::new();
        let volume = 0.02;
        backend.set_volume(volume);
        Self {
            queue: Vec::new(),
            now_playing: None,
            index: None,
            volume,
            backend,
        }
    }
    fn handle_events(&mut self, event: Event) {
        match event {
            Event::Add(songs) => self.add(songs),
            Event::ClearQueue => self.clear_queue(),
            Event::Remove(song) => self.remove(song),
            Event::Next => self.next(),
            Event::Previous => self.prev(),
            Event::Volume(v) => self.backend.set_volume(v),
            Event::TogglePlayback => self.backend.toggle_playback(),
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
            if let Some(elapsed) = now_playing.elapsed {
                if elapsed == 0.0 {
                    // *events = Event::Next;
                }
            }
        } else {
            //add the first song to the queue
            if let Some(song) = queue.first() {
                *now_playing = Some(song.clone());
                *index = Some(0);
                backend.play_file(&song.path);
            } else {
                //Nothing to do...
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
    fn next(&mut self) {
        let (now_playing, index, queue) = (&mut self.now_playing, &mut self.index, &self.queue);

        if let Some(song) = now_playing {
            if let Some(index) = index {
                if let Some(next_song) = queue.get(*index + 1) {
                    *song = next_song.clone();
                    *index += 1;
                } else if let Some(next_song) = queue.first() {
                    *song = next_song.clone();
                    *index = 0;
                }
            }
            let song = song.clone();
            self.backend.play_file(&song.path);
        }
    }
    fn prev(&mut self) {
        let (now_playing, index, queue) = (&mut self.now_playing, &mut self.index, &self.queue);

        if let Some(song) = now_playing {
            if let Some(index) = index {
                if *index != 0 {
                    if let Some(next_song) = queue.get(*index - 1) {
                        *song = next_song.clone();
                        *index -= 1;
                    }
                } else if let Some(next_song) = queue.last() {
                    *song = next_song.clone();
                    *index = queue.len() - 1;
                }
            }
            let song = song.clone();
            self.backend.play_file(&song.path);
        }
    }
    pub fn add(&mut self, songs: Vec<Song>) {
        self.queue.extend(songs);
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
    pub fn get_seeker(&self) -> String {
        format!("{}/{}", self.get_elapsed(), self.get_duration())
    }
    pub fn get_elapsed(&self) -> String {
        if let Some(song) = &self.now_playing {
            if let Some(elapsed) = song.elapsed {
                let mins = elapsed / 60.0;
                let rem = elapsed % 60.0;
                return format!(
                    "{:0width$}:{:0width$}",
                    mins.trunc() as usize,
                    rem.trunc() as usize,
                    width = 2,
                );
            }
        }
        String::from("00:00")
    }
    pub fn get_duration(&self) -> String {
        if let Some(song) = &self.now_playing {
            let mins = song.duration / 60.0;
            let rem = song.duration % 60.0;
            return format!(
                "{:0width$}:{:0width$}",
                mins.trunc() as usize,
                rem.trunc() as usize,
                width = 2,
            );
        }
        String::from("00:00")
    }
    pub fn get_volume(&self) -> String {
        self.volume.to_string()
    }
    pub fn get_queue(&self) -> Vec<String> {
        self.queue
            .iter()
            .map(|song| song.name_with_number.clone())
            .collect()
    }
}

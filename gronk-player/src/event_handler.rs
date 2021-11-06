use gronk_types::Song;

use crate::queue::{Queue, QueueSong};
use backend::Backend;

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
    queue: Queue,
    volume: f32,
    backend: Backend,
}

impl EventHandler {
    pub fn new() -> Self {
        let mut backend = Backend::new();
        let volume = 0.02;
        backend.set_volume(volume);
        Self {
            queue: Queue::new(),
            // queue: Queue::test(),
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
            Event::Volume(v) => self.set_volume(v),
            Event::TogglePlayback => self.backend.toggle_playback(),
            Event::Stop => self.backend.stop(),
            Event::Null => (),
        }
    }
    pub fn update(&mut self, event: Event) {
        self.handle_events(event);

        let Self {
            queue,
            volume: _,
            backend,
        } = self;

        //check if anything is playing
        if let Some(now_playing) = &mut queue.now_playing {
            //update the time elapsed
            if backend.is_playing() {
                let elapsed = backend.get_elapsed();
                let duration = backend.get_duration();
                now_playing.update(elapsed, duration);
                queue.percent = ((elapsed / duration * 100.0) as u16).clamp(0, 100);
            } else {
                self.next();
            }
        } else {
            //add the first song to the queue
            if let Some(song) = &mut queue.songs.first() {
                queue.now_playing = Some(song.clone());
                queue.index = Some(0);
                backend.play_file(&song.path);
            } else {
                //Nothing to do...
            }
        }
    }
    fn next(&mut self) {
        if let Some(song) = self.queue.next_song() {
            self.backend.play_file(&song);
        }
    }
    fn prev(&mut self) {
        if let Some(song) = self.queue.prev_song() {
            self.backend.play_file(&song);
        }
    }
    pub fn add(&mut self, songs: Vec<Song>) {
        let songs = QueueSong::from_vec(songs);
        self.queue.songs.extend(songs);
    }
    pub fn remove(&mut self, song: Song) {
        self.queue.songs = self
            .queue
            .songs
            .iter()
            .filter_map(|s| if s == &song { None } else { Some(s.to_owned()) })
            .collect();
    }
    pub fn clear_queue(&mut self) {
        self.queue.songs = Vec::new();
        self.queue.now_playing = None;
    }
    pub fn get_seeker(&self) -> String {
        format!("{}/{}", self.get_elapsed(), self.get_duration())
    }
    pub fn get_elapsed(&self) -> String {
        if let Some(song) = &self.queue.now_playing {
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
        if let Some(song) = &self.queue.now_playing {
            if let Some(duration) = &song.duration {
                let mins = duration / 60.0;
                let rem = duration % 60.0;
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
    pub fn get_volume(&self) -> String {
        self.volume.to_string()
    }
    pub fn set_volume(&mut self, v: f32) {
        self.volume += v;
        if self.volume > 0.1 {
            self.volume = 0.1;
        } else if self.volume < 0.0 {
            self.volume = 0.0;
        }
        self.backend.set_volume(self.volume);
    }
    pub fn get_queue(&self) -> Queue {
        self.queue.clone()
    }
}

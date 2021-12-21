use crate::index::Index;
use gronk_types::Song;
use rodio::Player;
use std::{path::PathBuf, time::Duration};

//this makes the code worse but easier?
pub struct List {
    pub songs: Vec<Song>,
    pub now_playing: Option<usize>,
}

impl List {
    pub fn new() -> Self {
        Self {
            songs: Vec::new(),
            now_playing: None,
        }
    }
    pub fn add(&mut self, other: &mut Vec<Song>) {
        self.songs.append(other);
    }
    pub fn next(&mut self) {
        if let Some(mut playing) = self.now_playing {
            if playing == self.songs.len() - 1 {
                playing = 0;
            } else {
                playing += 1;
            }
            self.now_playing = Some(playing);
        }
    }
    pub fn prev(&mut self) {
        if let Some(mut playing) = self.now_playing {
            if playing == 0 {
                playing = self.songs.len() - 1;
            } else {
                playing -= 1;
            }
            self.now_playing = Some(playing);
        }
    }
    pub fn playing(&self) -> Option<PathBuf> {
        if let Some(index) = self.now_playing {
            if let Some(song) = self.songs.get(index) {
                return Some(song.path.clone());
            }
        }
        None
    }
    pub fn clear(&mut self) {
        self.songs = Vec::new();
        self.now_playing = None;
    }
    pub fn len(&self) -> usize {
        self.songs.len()
    }
    pub fn remove(&mut self, index: usize) -> bool {
        self.songs.remove(index);
        if let Some(playing) = self.now_playing {
            //if the removed song was playing
            let len = self.songs.len();
            if len == 0 {
                self.clear();
            } else if playing == index && index == 0 {
                self.now_playing = Some(0);
            } else if playing == index && len == index {
                self.now_playing = Some(len - 1);
            } else if index < playing {
                self.now_playing = Some(playing - 1);
            } else if index > playing {
                //do nothing
                return false;
            }
            return true;
        }
        false
    }
    pub fn play(&mut self, index: usize) {
        self.now_playing = Some(index);
    }
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty() && self.now_playing.is_none()
    }
}

pub struct Queue {
    pub ui_index: Index,
    pub list: List,
    player: Player,
    skip_fix: bool,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            ui_index: Index::new(),
            list: List::new(),
            player: Player::new(),
            skip_fix: false,
        }
    }
    pub fn volume_up(&mut self) {
        self.player.set_volume(0.005);
    }
    pub fn volume_down(&mut self) {
        self.player.set_volume(-0.005);
    }
    pub fn play(&self) {
        if self.player.is_paused() {
            self.player.toggle_playback();
        }
    }
    pub fn play_pause(&self) {
        self.player.toggle_playback();
    }
    pub fn seeker(&self) -> f64 {
        self.player.seeker()
    }
    pub fn update(&mut self) {
        if !self.player.is_done() && self.skip_fix {
            self.next();
        }
        if self.player.is_done() {
            self.skip_fix = true;
        } else {
            self.skip_fix = false;
        }
    }
    pub fn prev(&mut self) {
        self.list.prev();
        self.play_selected();
    }
    pub fn next(&mut self) {
        self.list.next();
        self.play_selected();
    }
    pub fn clear(&mut self) {
        self.list.clear();
        self.ui_index.select(None);
        self.player.stop();
    }
    pub fn up(&mut self) {
        let len = self.list.len();
        self.ui_index.up(len);
    }
    pub fn down(&mut self) {
        let len = self.list.len();
        self.ui_index.down(len);
    }
    pub fn add(&mut self, mut songs: Vec<Song>) {
        //clippy will tell you this is wrong :/
        if self.list.is_empty() {
            self.list.add(&mut songs);
            self.list.now_playing = Some(0);
            self.ui_index.select(Some(0));
            self.play_selected();
        } else {
            self.list.add(&mut songs);
        }
    }
    pub fn select(&mut self) {
        if let Some(index) = self.ui_index.index {
            self.list.play(index);
            self.play_selected();
        }
    }
    pub fn delete_selected(&mut self) {
        if let Some(index) = self.ui_index.index {
            let update = self.list.remove(index);
            if index > self.list.len().saturating_sub(1) {
                self.ui_index.select(Some(self.list.len() - 1));
            }
            if update {
                self.play_selected();
            }
        }
    }
    pub fn play_selected(&mut self) {
        if let Some(path) = self.list.playing() {
            self.player.play(path.as_path());
        } else {
            self.player.stop();
        }
    }
    pub fn seek_fw(&mut self) {
        self.player.seek_fw();
    }
    pub fn seek_bw(&mut self) {
        self.player.seek_bw();
    }
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
    pub fn is_playing(&self) -> bool {
        !self.player.is_paused()
    }
    pub fn duration(&self) -> Option<Duration> {
        if self.is_empty() {
            None
        } else {
            self.player.duration()
        }
    }
    pub fn elapsed(&self) -> Duration {
        self.player.elapsed()
    }
    pub fn seek_to(&self, new_time: f64) {
        self.player.seek_to(Duration::from_secs_f64(new_time));
    }
    pub fn get_playing(&self) -> Option<&Song> {
        if let Some(i) = self.list.now_playing {
            self.list.songs.get(i)
        } else {
            None
        }
    }
    pub fn get_volume_percent(&self) -> u16 {
        self.player.volume_percent()
    }
}

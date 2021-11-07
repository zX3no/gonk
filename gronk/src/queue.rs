use gronk_player::Player;
use gronk_types::Song;
use std::path::PathBuf;

pub struct Queue {
    pub songs: Vec<Song>,
    pub ui_index: Option<usize>,
    pub now_playing: Option<usize>,
    player: Player,
    //this makes sure the song isn't skipped
    //while it's being loaded
    //this should be removed
    temp: bool,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            songs: Vec::new(),
            ui_index: None,
            now_playing: None,
            player: Player::new(),
            temp: false,
        }
    }
    pub fn volume_up(&mut self) {
        self.player.volume(0.01);
    }
    pub fn volume_down(&mut self) {
        self.player.volume(-0.01);
    }
    pub fn play_pause(&self) {
        self.player.toggle_playback();
    }
    pub fn update(&mut self) {
        if self.now_playing.is_some() {
            if !self.is_playing() && !self.songs.is_empty() && self.temp {
                self.next();
            }
        }
        if self.is_playing() {
            self.temp = true;
        } else {
            self.temp = false;
        }
    }
    pub fn is_playing(&self) -> bool {
        self.player.is_playing()
    }
    pub fn prev(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.now_playing {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }

            self.player.play(self.current_track());
        }
    }
    pub fn next(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.now_playing {
            if *index < len - 1 {
                *index += 1;
            } else {
                //TODO: check if this is working
                *index = 0;
            }
            self.player.play(self.current_track());
        }
    }
    pub fn current_track(&self) -> PathBuf {
        self.songs
            .get(self.now_playing.unwrap())
            .unwrap()
            .path
            .clone()
    }
    pub fn clear(&mut self) {
        self.songs = Vec::new();
        self.now_playing = None;
        self.ui_index = None;
        self.temp = false;
        self.player.stop();
    }
    pub fn add(&mut self, mut songs: Vec<Song>) {
        self.songs.append(&mut songs);
        if self.now_playing.is_none() && !self.songs.is_empty() {
            self.now_playing = Some(0);
            self.ui_index = Some(0);
            let song = self.songs.first().unwrap();
            self.player.play(song.path.clone());
        }
    }
    pub fn up(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.ui_index {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }
        }
    }
    pub fn down(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.ui_index {
            if *index + 1 < len {
                *index += 1;
            } else {
                *index = 0;
            }
        }
    }
    pub fn play_selected(&mut self) {
        if let Some(index) = self.ui_index {
            if let Some(song) = self.songs.get(index) {
                self.player.play(song.path.clone());
                self.now_playing = Some(index);
            }
        }
    }
}

//TODO: find a way to actually use this
pub trait UpDown {
    fn up(index: &mut Option<usize>, len: usize) {
        if let Some(index) = index {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }
        }
    }
    fn down(index: &mut Option<usize>, len: usize) {
        if let Some(index) = index {
            if *index + 1 < len {
                *index += 1;
            } else {
                *index = 0;
            }
        }
    }
}

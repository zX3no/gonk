use gronk_types::Song;

use crate::player::Player;

pub struct Queue {
    pub index: Option<usize>,
    pub songs: Vec<Song>,
    player: Player,
}
impl Queue {
    pub fn new() -> Self {
        Self {
            index: None,
            songs: Vec::new(),
            player: Player::new(),
        }
    }
    pub fn get_queue(&self) -> (&Vec<Song>, &Option<usize>) {
        (&self.songs, &self.index)
    }
    pub fn volume_up(&mut self) {
        self.player.volume(0.005);
    }
    pub fn volume_down(&mut self) {
        self.player.volume(-0.005);
    }
    pub fn play_pause(&self) {
        self.player.toggle_playback();
    }
    pub fn next(&self) {}
    pub fn prev(&self) {}
    pub fn clear(&mut self) {
        self.songs = Vec::new();
        self.player.stop();
    }
    pub fn add(&mut self, mut songs: Vec<Song>) {
        self.songs.append(&mut songs);
    }
    pub fn up(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.index {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }
        }
    }
    pub fn down(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.index {
            if *index + 1 < len {
                *index += 1;
            } else {
                *index = 0;
            }
        }
    }
}

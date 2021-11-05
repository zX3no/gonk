use gronk_player::player::Player;
use std::path::PathBuf;

//could probably remove this struct and put contents into music

//Some changes to the old version:
//the queue is stored in sql instead of a struct
//the only information i need to gather from soloud is
//the duration which should be held as (elapsed, duration)
//the playing song should be set in the database
//I don't like the way this implies the player is talking to the database
//i suppose they queue can be stored twice?
//once in the database and once in the player

pub struct Queue {
    player: Player,
}
impl Queue {
    pub fn new() -> Self {
        Self {
            player: Player::new(),
        }
    }
    pub fn add(&self, songs: Vec<PathBuf>) {
        self.player.add(songs);
    }
    pub fn remove(&self, song: PathBuf) {
        self.player.remove(song);
    }
    pub fn next(&self) {
        self.player.next();
    }
    pub fn prev(&self) {
        self.player.previous();
    }
    pub fn clear(&self) {
        self.player.clear_queue();
        self.player.stop()
    }
    pub fn pause(&self) {
        self.player.toggle_playback();
    }
    pub fn volume_down(&self) {
        self.player.volume(-0.005);
    }
    pub fn volume_up(&self) {
        self.player.volume(0.005);
    }
}

use gronk_player::Player;
use gronk_types::Song;

pub struct Queue {
    pub index: Option<usize>,
    pub songs: Vec<Song>,
    now_playing: Option<usize>,
    player: Player,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            index: None,
            songs: Vec::new(),
            now_playing: None,
            player: Player::new(),
        }
    }
    pub fn get_queue(&self) -> (&Vec<Song>, &Option<usize>) {
        (&self.songs, &self.index)
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
            if !self.is_playing() {
                self.next();
            }
        }
    }
    pub fn is_playing(&self) -> bool {
        self.player.is_playing()
    }
    pub fn next(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.now_playing {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }
        }
    }
    pub fn prev(&mut self) {
        let len = self.songs.len();
        if let Some(index) = &mut self.now_playing {
            if *index + 1 < len {
                *index += 1;
            } else {
                *index = 0;
            }
        }
    }
    pub fn clear(&mut self) {
        self.songs = Vec::new();
        self.now_playing = None;
        self.player.stop();
    }
    pub fn add(&mut self, mut songs: Vec<Song>) {
        self.songs.append(&mut songs);
        if self.now_playing.is_none() && !self.songs.is_empty() {
            self.now_playing = Some(0);
            let song = self.songs.first().unwrap();
            self.player.play(song.path.clone())
        }
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

//?
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

use crate::music::Music;

pub enum Mode {
    Artist,
    Album,
    Song,
}

impl Mode {
    fn add(&mut self) {
        match self {
            Mode::Artist => *self = Mode::Album,
            Mode::Album => *self = Mode::Song,
            Mode::Song => *self = Mode::Artist,
        }
    }
    fn min(&mut self) {
        match self {
            Mode::Artist => *self = Mode::Song,
            Mode::Album => *self = Mode::Artist,
            Mode::Song => *self = Mode::Album,
        }
    }
}

pub struct App {
    pub music: Music,
    pub mode: Mode,
}

impl App {
    pub fn new() -> Self {
        Self {
            music: Music::new(),
            mode: Mode::Artist,
        }
    }

    pub fn next_mode(&mut self) {
        self.mode.add();
    }

    pub fn prev_mode(&mut self) {
        self.mode.min();
    }

    pub fn move_down(&mut self) {
        match self.mode {
            Mode::Artist => self.music.artists_down(),
            Mode::Album => self.music.albums_down(),
            Mode::Song => self.music.songs_down(),
        }
    }

    pub fn move_up(&mut self) {
        match self.mode {
            Mode::Artist => self.music.artists_up(),
            Mode::Album => self.music.albums_up(),
            Mode::Song => self.music.songs_up(),
        }
    }
}

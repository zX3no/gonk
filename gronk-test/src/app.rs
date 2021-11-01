use crate::types::Music;

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
            Mode::Artist => *self = Mode::Album,
            Mode::Album => *self = Mode::Song,
            Mode::Song => *self = Mode::Artist,
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
}

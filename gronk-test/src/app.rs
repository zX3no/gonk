use crate::music::Music;

pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

impl BrowserMode {
    fn add(&mut self) {
        match self {
            BrowserMode::Artist => *self = BrowserMode::Album,
            BrowserMode::Album => *self = BrowserMode::Song,
            BrowserMode::Song => (),
        }
    }
    fn min(&mut self) {
        match self {
            BrowserMode::Artist => (),
            BrowserMode::Album => *self = BrowserMode::Artist,
            BrowserMode::Song => *self = BrowserMode::Album,
        }
    }
}
pub enum Mode {
    Browser,
    Queue,
}

pub struct App {
    pub music: Music,
    pub browser_mode: BrowserMode,
    pub ui_mode: Mode,
}

impl App {
    pub fn new() -> Self {
        Self {
            music: Music::new(),
            browser_mode: BrowserMode::Artist,
            ui_mode: Mode::Browser,
        }
    }

    pub fn next_mode(&mut self) {
        self.browser_mode.add();
    }

    pub fn prev_mode(&mut self) {
        self.browser_mode.min();
    }

    pub fn move_down(&mut self) {
        match self.browser_mode {
            BrowserMode::Artist => self.music.artists_down(),
            BrowserMode::Album => self.music.albums_down(),
            BrowserMode::Song => self.music.songs_down(),
        }
    }

    pub fn move_up(&mut self) {
        match self.browser_mode {
            BrowserMode::Artist => self.music.artists_up(),
            BrowserMode::Album => self.music.albums_up(),
            BrowserMode::Song => self.music.songs_up(),
        }
    }
}

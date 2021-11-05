use crate::{music::Music, queue::Queue};

pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

impl BrowserMode {
    fn next(&mut self) {
        match self {
            BrowserMode::Artist => *self = BrowserMode::Album,
            BrowserMode::Album => *self = BrowserMode::Song,
            BrowserMode::Song => (),
        }
    }
    fn prev(&mut self) {
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
impl Mode {
    fn toggle(&mut self) {
        match self {
            Mode::Browser => *self = Mode::Queue,
            Mode::Queue => *self = Mode::Browser,
        }
    }
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
    pub fn add_to_queue(&mut self) {
        match self.browser_mode {
            BrowserMode::Artist => self.music.queue_artist(),
            BrowserMode::Album => self.music.queue_album(),
            BrowserMode::Song => self.music.queue_song(),
        }
    }
    pub fn ui_toggle(&mut self) {
        self.ui_mode.toggle();
    }

    pub fn browser_next(&mut self) {
        self.browser_mode.next();
    }

    pub fn browser_prev(&mut self) {
        self.browser_mode.prev();
    }

    pub fn browser_up(&mut self) {
        self.music.up(&self.browser_mode);
    }

    pub fn browser_down(&mut self) {
        self.music.down(&self.browser_mode);
    }
}

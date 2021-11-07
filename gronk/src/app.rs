use gronk_database::Database;

use crate::{music::Music, queue::Queue};

#[derive(Debug)]
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
    //TODO: Search mode
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
    pub queue: Queue,
    database: Database,
    pub browser_mode: BrowserMode,
    pub ui_mode: Mode,
}

impl App {
    pub fn new() -> Self {
        let database = Database::new();
        Self {
            music: Music::new(&database),
            queue: Queue::new(),
            database,
            browser_mode: BrowserMode::Artist,
            ui_mode: Mode::Browser,
        }
    }
    pub fn ui_toggle(&mut self) {
        self.ui_mode.toggle();
    }
    pub fn browser_next(&mut self) {
        if let Mode::Browser = self.ui_mode {
            self.browser_mode.next();
        }
    }
    pub fn browser_prev(&mut self) {
        if let Mode::Browser = self.ui_mode {
            self.browser_mode.prev();
        }
    }
    pub fn up(&mut self) {
        match self.ui_mode {
            Mode::Browser => self.music.up(&self.browser_mode, &self.database),
            Mode::Queue => self.queue.up(),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode {
            Mode::Browser => self.music.down(&self.browser_mode, &self.database),
            Mode::Queue => self.queue.down(),
        }
    }
    pub fn update_db(&self) {
        todo!();
    }
    pub fn add_to_queue(&mut self) {
        match self.ui_mode {
            Mode::Browser => {
                let (artist, album, track, song) = (
                    &self.music.selected_artist.item,
                    &self.music.selected_album.item,
                    self.music.selected_song.prefix.unwrap(),
                    &self.music.selected_song.item,
                );
                let songs = match self.browser_mode {
                    BrowserMode::Artist => self.database.get_artist(&artist),
                    BrowserMode::Album => self.database.get_album(&artist, &album),
                    BrowserMode::Song => self.database.get_song(&artist, &album, &track, &song),
                };
                self.queue.add(songs);
            }
            Mode::Queue => {
                self.queue.play_selected();
            }
        }
    }

    pub fn on_tick(&mut self) {
        self.queue.update()
    }
}

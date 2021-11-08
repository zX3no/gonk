use crate::{music::Music, queue::Queue};
use crossterm::event::{KeyCode, KeyModifiers};
use gronk_database::Database;

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
    Search,
}

impl Mode {
    fn toggle(&mut self) {
        match self {
            Mode::Browser => *self = Mode::Queue,
            Mode::Queue => *self = Mode::Browser,
            Mode::Search => *self = Mode::Queue,
        }
    }
}

pub struct App {
    pub music: Music,
    pub queue: Queue,
    database: Database,
    pub browser_mode: BrowserMode,
    pub ui_mode: Mode,
    pub query: String,
    pub constraint: [u16; 4],
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
            query: String::new(),
            constraint: [8, 42, 24, 26],
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
            _ => (),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode {
            Mode::Browser => self.music.down(&self.browser_mode, &self.database),
            Mode::Queue => self.queue.down(),
            _ => (),
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
                    self.music.selected_song.prefix.as_ref().unwrap(),
                    &self.music.selected_song.item,
                );
                // dbg!(song);
                // dbg!(self.music.selected_song.index);

                let songs = match self.browser_mode {
                    BrowserMode::Artist => self.database.get_artist(artist),
                    BrowserMode::Album => self.database.get_album(artist, album),
                    BrowserMode::Song => self.database.get_song(artist, album, track, song),
                };
                self.queue.add(songs);
            }
            Mode::Queue => {
                self.queue.play_selected();
            }
            _ => (),
        }
    }
    pub fn on_tick(&mut self) {
        self.queue.update()
    }
    pub fn ui_search(&mut self) {
        self.ui_mode = Mode::Search;
    }
    //TODO: change this to song for pretty search printing
    pub fn get_search(&self) -> Option<Vec<String>> {
        self.database.search(&self.query)
    }
    // pub fn get_search_custom(&self) -> Option<Vec<String>> {
    //     if self.query.is_empty() {
    //         None
    //     } else {
    //         let songs = self.database.get_all_songs();
    //         let fzf = songs.fzf(&self.query);
    //         fzf
    //     }
    // }
    pub fn search_event(&mut self, code: KeyCode, modifier: KeyModifiers) {
        match code {
            KeyCode::Char(c) => {
                self.query.push(c);
            }
            KeyCode::Tab => self.ui_mode = Mode::Browser,
            KeyCode::Backspace => {
                if modifier == KeyModifiers::CONTROL {
                    let rev: String = self.query.chars().rev().collect();
                    if let Some(index) = rev.find(' ') {
                        let len = self.query.len();
                        let str = self.query.split_at(len - index - 1);
                        self.query = str.0.to_string();
                    } else {
                        self.query = String::new();
                    }
                } else {
                    self.query.pop();
                }
            }
            _ => (),
        }
    }
    pub fn move_constraint(&mut self, arg: char, modifier: KeyModifiers) {
        let i = (arg as usize) - 49;
        if modifier == KeyModifiers::SHIFT {
            if self.constraint[i] != 0 {
                self.constraint[i] = self.constraint[i].saturating_sub(1);
                self.constraint[i + 1] += 1;
            }
        } else {
            if self.constraint[i + 1] != 0 {
                self.constraint[i] += 1;
                self.constraint[i + 1] = self.constraint[i + 1].saturating_sub(1);
            }
        }
        for n in &mut self.constraint {
            if *n > 100 {
                *n = 100;
            }
        }
        if self.constraint.iter().sum::<u16>() != 100 {
            panic!("{:?}", self.constraint);
        }
    }
}

use browser::Browser;
use crossterm::event::{KeyCode, KeyModifiers};
use gronk_database::Database;
use queue::Queue;
use search::Search;

use crate::modes::{BrowserMode, UiMode};

mod browser;
mod queue;
mod search;

pub struct App {
    pub music: Browser,
    pub queue: Queue,
    pub search: Search,
    database: Database,
    pub browser_mode: BrowserMode,
    pub ui_mode: UiMode,
    pub constraint: [u16; 4],
}

impl App {
    pub fn new() -> Self {
        let database = Database::new();
        Self {
            music: Browser::new(&database),
            queue: Queue::new(),
            search: Search::new(),
            database,
            browser_mode: BrowserMode::Artist,
            ui_mode: UiMode::Browser,
            constraint: [8, 42, 24, 26],
        }
    }
    pub fn ui_toggle(&mut self) {
        self.ui_mode.toggle();
    }
    pub fn browser_next(&mut self) {
        if let UiMode::Browser = self.ui_mode {
            self.browser_mode.next();
        }
    }
    pub fn browser_prev(&mut self) {
        if let UiMode::Browser = self.ui_mode {
            self.browser_mode.prev();
        }
    }
    pub fn up(&mut self) {
        match self.ui_mode {
            UiMode::Browser => self.music.up(&self.browser_mode, &self.database),
            UiMode::Queue => self.queue.up(),
            _ => (),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode {
            UiMode::Browser => self.music.down(&self.browser_mode, &self.database),
            UiMode::Queue => self.queue.down(),
            _ => (),
        }
    }
    pub fn update_db(&self) {
        todo!();
    }
    pub fn add_to_queue(&mut self) {
        match self.ui_mode {
            UiMode::Browser => {
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
            UiMode::Queue => {
                self.queue.play_selected();
            }
            _ => (),
        }
    }
    pub fn on_tick(&mut self) {
        self.queue.update()
    }
    pub fn ui_search(&mut self) {
        self.ui_mode = UiMode::Search;
    }
    //TODO: change this to song for pretty search printing
    pub fn get_search(&self) -> Option<Vec<String>> {
        self.database.search(&self.search.query)
    }
    pub fn search_event(&mut self, code: KeyCode, modifier: KeyModifiers) {
        match code {
            KeyCode::Char(c) => {
                self.search.query.push(c);
            }
            KeyCode::Tab => self.ui_mode = UiMode::Browser,
            KeyCode::Backspace => {
                if modifier == KeyModifiers::CONTROL {
                    let rev: String = self.search.query.chars().rev().collect();
                    if let Some(index) = rev.find(' ') {
                        let len = self.search.query.len();
                        let str = self.search.query.split_at(len - index - 1);
                        self.search.query = str.0.to_string();
                    } else {
                        self.search.query = String::new();
                    }
                } else {
                    self.search.query.pop();
                }
            }
            _ => (),
        }
    }
    pub fn move_constraint(&mut self, arg: char, modifier: KeyModifiers) {
        //1 is 48, '1' - 49 = 0
        let i = (arg as usize) - 49;
        if modifier == KeyModifiers::SHIFT && self.constraint[i] != 0 {
            self.constraint[i] = self.constraint[i].saturating_sub(1);
            self.constraint[i + 1] += 1;
        } else if self.constraint[i + 1] != 0 {
            self.constraint[i] += 1;
            self.constraint[i + 1] = self.constraint[i + 1].saturating_sub(1);
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

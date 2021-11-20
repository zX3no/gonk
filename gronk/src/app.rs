use crate::modes::{BrowserMode, Mode, UiMode};
use browser::Browser;
use crossterm::event::KeyModifiers;
use gronk_database::Database;
use gronk_types::Song;
use queue::Queue;
use search::Search;

mod browser;
mod queue;
mod search;

pub struct App {
    pub music: Browser,
    pub queue: Queue,
    pub search: Search,
    database: Option<Database>,
    pub browser_mode: BrowserMode,
    pub ui_mode: Mode,
    pub constraint: [u16; 4],
    pub seeker: u16,
}

impl App {
    pub fn new() -> Self {
        let database = Database::new();
        let songs = database.get_songs();
        let music = Browser::new(&database);
        let queue = Queue::new();
        let search = Search::new(&songs);

        Self {
            music,
            queue,
            search,
            database: Some(database),
            browser_mode: BrowserMode::Artist,
            ui_mode: Mode::new(),
            seeker: 0,
            //this could be [8, 42, 24, 100]
            constraint: [8, 42, 24, 26],
        }
    }
    pub fn browser_next(&mut self) {
        if self.ui_mode == UiMode::Browser {
            self.browser_mode.next();
        }
    }
    pub fn browser_prev(&mut self) {
        if self.ui_mode == UiMode::Browser {
            self.browser_mode.prev();
        }
    }
    pub fn up(&mut self) {
        match self.ui_mode.current {
            UiMode::Browser => self
                .music
                .up(&self.browser_mode, &self.database.as_ref().unwrap()),
            UiMode::Queue => self.queue.up(),
            _ => (),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode.current {
            UiMode::Browser => self
                .music
                .down(&self.browser_mode, &self.database.as_ref().unwrap()),
            UiMode::Queue => self.queue.down(),
            _ => (),
        }
    }
    pub fn on_enter(&mut self) {
        match self.ui_mode.current {
            UiMode::Browser => {
                let (artist, album, track, song) = (
                    &self.music.selected_artist.item,
                    &self.music.selected_album.item,
                    self.music.selected_song.prefix.as_ref().unwrap(),
                    &self.music.selected_song.item,
                );

                if let Some(db) = &self.database {
                    let songs = match self.browser_mode {
                        BrowserMode::Artist => db.get_artist(artist),
                        BrowserMode::Album => db.get_album(artist, album),
                        BrowserMode::Song => db.get_song(artist, album, track, song),
                    };
                    self.queue.add(songs);
                }
            }
            UiMode::Queue => {
                self.queue.select();
            }
            _ => (),
        }
    }
    pub fn on_tick(&mut self) {
        self.queue.update()
    }
    pub fn search(&mut self) -> Vec<Song> {
        if self.search.changed() {
            self.search.update_search();
        }
        let ids = &self.search.results;

        if let Some(db) = &self.database {
            db.get_songs_from_ids(&ids)
        } else {
            Vec::new()
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
    pub fn handle_input(&mut self, c: char, modifier: KeyModifiers) {
        if self.ui_mode.current == UiMode::Search {
            self.search.push(c);
        } else {
            match c {
                'c' => self.queue.clear(),
                'j' => self.down(),
                'k' => self.up(),
                'h' => self.browser_prev(),
                'l' => self.browser_next(),
                ' ' => self.queue.play_pause(),
                'a' => self.queue.prev(),
                'd' => self.queue.next(),
                'w' => self.queue.volume_up(),
                's' => self.queue.volume_down(),
                '/' => self.ui_mode.update(UiMode::Search),
                'x' => self.delete_from_queue(),
                '1' | '!' => self.move_constraint('1', modifier),
                '2' | '@' => self.move_constraint('2', modifier),
                '3' | '#' => self.move_constraint('3', modifier),
                _ => (),
            }
        }
    }
    pub fn on_tab(&mut self) {
        if self.ui_mode == UiMode::Search {
            self.ui_mode.flip()
        } else {
            self.ui_mode.toggle();
        }
    }
    pub fn on_backspace(&mut self, modifier: KeyModifiers) {
        if modifier == KeyModifiers::CONTROL {
            //TODO: maybe just reset the whole query
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
    fn delete_from_queue(&mut self) {
        self.queue.delete_selected();
    }
    pub fn update(&mut self) {
        self.database = None;
        Database::create_db().unwrap();
        self.database = Some(Database::new());
    }
}

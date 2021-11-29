use std::{path::Path, time::Duration};

use crate::modes::{BrowserMode, Mode, SearchMode, UiMode};
use browser::Browser;
use crossterm::event::{KeyCode, KeyModifiers};
use gronk_database::Database;
use gronk_types::Song;
use queue::Queue;
use search::Search;

mod browser;
mod queue;
mod search;

pub struct App {
    pub browser: Browser,
    pub queue: Queue,
    pub search: Search,
    database: Option<Database>,
    //TODO: why are these modes so confusing
    pub browser_mode: BrowserMode,
    pub ui_mode: Mode,
    pub constraint: [u16; 4],
    pub seeker: f64,
}

impl App {
    pub fn new() -> Self {
        let database = Database::new(vec![&Path::new("D:\\Music")]).unwrap();
        let songs = database.get_songs();
        let music = Browser::new(&database);
        let queue = Queue::new();
        let search = Search::new(&songs);

        Self {
            browser: music,
            queue,
            search,
            database: Some(database),
            browser_mode: BrowserMode::Artist,
            ui_mode: Mode::new(),
            seeker: 0.0,
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
                .browser
                .up(&self.browser_mode, self.database.as_ref().unwrap()),
            UiMode::Queue => self.queue.up(),
            UiMode::Search => self.search.up(),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode.current {
            UiMode::Browser => self
                .browser
                .down(&self.browser_mode, self.database.as_ref().unwrap()),
            UiMode::Queue => self.queue.down(),
            UiMode::Search => self.search.down(),
        }
    }
    pub fn on_enter(&mut self) {
        match self.ui_mode.current {
            UiMode::Browser => {
                let (artist, album, track, song) = (
                    &self.browser.selected_artist.item,
                    &self.browser.selected_album.item,
                    self.browser.selected_song.prefix.as_ref().unwrap(),
                    &self.browser.selected_song.item,
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
            UiMode::Search => {
                let search = &mut self.search;
                if let SearchMode::Search = search.mode {
                    if !search.query.is_empty() && !search.results.is_empty() {
                        search.mode.next();
                        search.index.select(Some(0));
                    }
                } else if let Some(index) = search.get_selected() {
                    let songs = self.search();
                    let song = songs.get(index).unwrap();
                    self.queue.add(vec![song.clone()]);
                }
            }
        }
    }
    pub fn on_tick(&mut self) {
        self.queue.update();

        self.seeker = self.queue.seeker();
    }
    pub fn search(&mut self) -> Vec<Song> {
        if self.search.changed() {
            self.search.update_search();
        }
        let ids = &self.search.results;

        if let Some(db) = &self.database {
            db.get_songs_from_ids(ids)
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
    fn delete_from_queue(&mut self) {
        self.queue.delete_selected();
    }
    pub fn update(&mut self) {
        // self.database = None;
        // self.database = Some(Database::new(PathBuf::from("D:\\Music")).unwrap());
        todo!();
    }
    pub fn input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Char(c) => self.handle_char(c, modifiers),
            KeyCode::Down => self.down(),
            KeyCode::Up => self.up(),
            KeyCode::Left => self.browser_prev(),
            KeyCode::Right => self.browser_next(),
            KeyCode::Enter => self.on_enter(),
            KeyCode::Tab => {
                if self.ui_mode == UiMode::Search {
                    self.search.reset();
                    self.ui_mode.flip();
                } else {
                    self.ui_mode.toggle();
                }
            }
            KeyCode::Backspace => match self.search.mode {
                SearchMode::Search => {
                    if modifiers == KeyModifiers::CONTROL {
                        self.search.query = String::new();
                    } else {
                        self.search.query.pop();
                    }
                }
                SearchMode::Select => self.search.exit(),
            },
            KeyCode::Esc => {
                if self.ui_mode == UiMode::Search {
                    self.search.exit();
                }
            }
            _ => (),
        }
    }
    pub fn handle_char(&mut self, c: char, modifier: KeyModifiers) {
        if self.ui_mode.current == UiMode::Search {
            self.search.on_key(c);
        } else {
            match c {
                'u' => self.update(),
                'c' => self.queue.clear(),
                'j' => self.down(),
                'k' => self.up(),
                'h' => self.browser_prev(),
                'l' => self.browser_next(),
                ' ' => self.queue.play_pause(),
                'q' => self.queue.seek(Duration::from_secs_f32(-10.0)),
                'e' => self.queue.seek(Duration::from_secs(10)),
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
}

use std::path::Path;

use crate::modes::{BrowserMode, Mode, SearchMode, UiMode};
use browser::Browser;
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use gronk_database::Database;
use gronk_search::{ItemType, SearchItem};
use queue::Queue;
use search::Search;

mod browser;
mod queue;
mod search;

pub struct App {
    pub browser: Browser,
    pub queue: Queue,
    pub search: Search,
    pub database: Option<Database>,
    //TODO: why are these modes so confusing
    pub browser_mode: BrowserMode,
    pub ui_mode: Mode,
    pub constraint: [u16; 4],
    pub seeker: f64,
}

impl App {
    pub fn new() -> Self {
        let database = Database::new().unwrap();

        //check if user wants to add new database
        let args: Vec<_> = std::env::args().skip(1).collect();
        if let Some(first) = args.first() {
            if first == "add" {
                if let Some(dir) = args.get(1) {
                    database.add_dir(dir);
                }
            }
        }

        let music = Browser::new(&database);
        let queue = Queue::new();

        let songs = database.get_songs();
        let artists = database.artists().unwrap();
        let albums = database.albums();
        let search = Search::new(&songs, &albums, &artists);

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
                        BrowserMode::Album => db.get_album(album, artist),
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
                    if !search.is_empty() {
                        search.mode.next();
                        search.index.select(Some(0));
                    }
                } else if let Some(selected) = search.get_selected() {
                    let db = self.database.as_ref().unwrap();
                    match selected.item_type {
                        ItemType::Song => {
                            let song = db.get_song_from_id(selected.song_id.unwrap());
                            self.queue.add(vec![song.clone()]);
                        }
                        ItemType::Album => {
                            let songs = db
                                .get_album(&selected.name, selected.album_artist.as_ref().unwrap());
                            self.queue.add(songs);
                        }
                        ItemType::Artist => {
                            let songs = db.get_artist(&selected.name);
                            self.queue.add(songs);
                        }
                    }
                }
            }
        }
    }
    pub fn on_tick(&mut self) {
        self.queue.update();

        self.seeker = self.queue.seeker();

        //update search results
        if self.search.query_changed() {
            self.search.update_search();
        }
    }
    pub fn get_search(&self) -> &Vec<SearchItem> {
        &self.search.results
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
    pub fn reset_db(&mut self) {
        if let Some(db) = &mut self.database {
            db.reset(vec![&Path::new("D:\\Music")])
        }
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
                SearchMode::Search => self.search.on_backspace(modifiers),
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
                'u' => self.reset_db(),
                'c' => self.queue.clear(),
                'j' => self.down(),
                'k' => self.up(),
                'h' => self.browser_prev(),
                'l' => self.browser_next(),
                ' ' => self.queue.play_pause(),
                'q' => self.queue.seek_bw(),
                'e' => self.queue.seek_fw(),
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
    pub fn mouse(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::ScrollDown => self.down(),
            MouseEventKind::ScrollUp => self.up(),
            _ => (),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

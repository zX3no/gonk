use crate::modes::{BrowserMode, SearchMode, UiMode};
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use gronk_database::Database;
use gronk_search::ItemType;

pub use {browser::Browser, queue::Queue, search::Search};

mod browser;
mod queue;
mod search;

pub struct App<'a> {
    pub browser: Browser<'a>,
    pub queue: Queue,
    pub search: Search,
    pub database: &'a Database,
    pub ui_mode: UiMode,
    //TODO: move these in proper structs?
}

impl<'a> App<'a> {
    pub fn new(db: &'a Database) -> Self {
        let music = Browser::new(&db);
        let queue = Queue::new();

        let songs = db.get_songs();
        let artists = db.artists().unwrap();
        let albums = db.albums();
        let search = Search::new(&songs, &albums, &artists);

        Self {
            browser: music,
            queue,
            search,
            database: db,
            ui_mode: UiMode::Browser,
        }
    }
    pub fn browser_next(&mut self) {
        if self.ui_mode == UiMode::Browser {
            self.browser.next();
        }
    }
    pub fn browser_prev(&mut self) {
        if self.ui_mode == UiMode::Browser {
            self.browser.prev();
        }
    }
    pub fn up(&mut self) {
        match self.ui_mode {
            UiMode::Browser => self.browser.up(),
            UiMode::Queue => self.queue.up(),
            UiMode::Search => self.search.up(),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode {
            UiMode::Browser => self.browser.down(),
            UiMode::Queue => self.queue.down(),
            UiMode::Search => self.search.down(),
        }
    }
    pub fn on_enter(&mut self) {
        match self.ui_mode {
            UiMode::Browser => {
                let (artist, album, track, song) = (
                    &self.browser.selected_artist.item,
                    &self.browser.selected_album.item,
                    self.browser.selected_song.prefix.as_ref().unwrap(),
                    &self.browser.selected_song.item,
                );

                let db = self.database;
                let songs = match self.browser.mode() {
                    BrowserMode::Artist => db.get_artist(artist),
                    BrowserMode::Album => db.get_album(album, artist),
                    BrowserMode::Song => db.get_song(artist, album, track, song),
                };
                self.queue.add(songs);
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
                    let db = self.database;
                    match selected.item_type {
                        ItemType::Song => {
                            let song = db.get_song_from_id(selected.song_id.unwrap());
                            self.queue.add(vec![song]);
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

        //update search results
        if self.search.query_changed() {
            self.search.update_search();
        }
    }
    fn delete_from_queue(&mut self) {
        self.queue.delete_selected();
    }
    pub fn reset_db(&mut self) {
        self.database.reset();
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
                self.ui_mode = match self.ui_mode {
                    UiMode::Browser => UiMode::Queue,
                    UiMode::Queue => UiMode::Browser,
                    UiMode::Search => {
                        self.search.reset();
                        UiMode::Queue
                    }
                };
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
        if self.ui_mode == UiMode::Search {
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
                '/' => self.ui_mode = UiMode::Search,
                'x' => self.delete_from_queue(),
                '1' | '!' => self.queue.move_constraint('1', modifier),
                '2' | '@' => self.queue.move_constraint('2', modifier),
                '3' | '#' => self.queue.move_constraint('3', modifier),
                _ => (),
            }
        }
    }
    pub fn mouse(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::ScrollDown => self.down(),
            MouseEventKind::ScrollUp => self.up(),
            MouseEventKind::Down(_) => {
                self.queue.clicked_pos = Some((event.column, event.row));
            }
            _ => (),
        }
    }
}

use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use gonk_database::{Database, Key, Toml};
pub use {browser::Browser, options::Options, queue::Queue, search::Search};

pub mod browser;
mod options;
mod queue;
mod search;

#[derive(PartialEq)]
pub enum AppMode {
    Browser,
    Queue,
    Search,
    Options,
}

pub struct App<'a> {
    pub db: &'a Database,
    pub browser: Browser<'a>,
    pub queue: Queue,
    pub search: Search<'a>,
    pub options: Options,
    pub app_mode: AppMode,
}

impl<'a> App<'a> {
    pub fn new(db: &'a Database) -> Self {
        let toml = Toml::new().unwrap();
        let music = Browser::new(db);
        let queue = Queue::new(toml.volume());
        let search = Search::new(db);
        let options = Options::new(toml);

        Self {
            browser: music,
            queue,
            search,
            options,
            db,
            app_mode: AppMode::Browser,
        }
    }
    fn browser_next(&mut self) {
        if self.app_mode == AppMode::Browser {
            self.browser.next();
        }
    }
    fn browser_prev(&mut self) {
        if self.app_mode == AppMode::Browser {
            self.browser.prev();
        }
    }
    fn up(&mut self) {
        match self.app_mode {
            AppMode::Browser => self.browser.up(),
            AppMode::Queue => self.queue.up(),
            AppMode::Search => self.search.up(),
            AppMode::Options => self.options.up(),
        }
    }
    fn down(&mut self) {
        match self.app_mode {
            AppMode::Browser => self.browser.down(),
            AppMode::Queue => self.queue.down(),
            AppMode::Search => self.search.down(),
            AppMode::Options => self.options.down(),
        }
    }
    fn on_enter(&mut self) {
        match self.app_mode {
            AppMode::Browser => {
                let songs = self.browser.on_enter();
                self.queue.add(songs);
            }
            AppMode::Queue => self.queue.select(),
            AppMode::Search => {
                if let Some(songs) = self.search.get_songs() {
                    self.queue.add(songs);
                }
            }
            AppMode::Options => {
                if let Some(dir) = self.options.on_enter(&mut self.queue) {
                    self.db.delete_path(&dir);
                    self.refresh_ui();
                }
            }
        }
    }
    fn on_escape(&mut self) {
        match self.app_mode {
            AppMode::Search => {
                if self.search.on_escape() {
                    //TODO previous mode should be stored
                    //self.app_mode = self.app_mode.last()
                    self.app_mode = AppMode::Queue;
                }
            }
            AppMode::Options => {
                self.app_mode = AppMode::Queue;
            }
            _ => (),
        }
    }
    fn on_tab(&mut self) {
        self.app_mode = match self.app_mode {
            AppMode::Browser => AppMode::Queue,
            AppMode::Queue => AppMode::Browser,
            AppMode::Search => {
                self.search.reset();
                AppMode::Queue
            }
            AppMode::Options => AppMode::Queue,
        };
    }
    pub fn on_tick(&mut self) {
        self.queue.update();

        if let Some(busy) = self.db.is_busy() {
            if busy {
                self.refresh_ui();
            }
            self.browser.update_busy(busy);
        }

        if self.search.has_query_changed() {
            self.search.update_search();
        }
    }
    fn delete_from_queue(&mut self) {
        if let AppMode::Queue = self.app_mode {
            self.queue.delete_selected();
        }
    }
    pub fn input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // let hk = Toml::new().unwrap().hotkey;

        // let (k, m) = (hk.change_mode.key, hk.change_mode.modifiers);

        // let k: KeyCode = k.into();

        // if k == code && m.is_none() {
        //     //Do something
        // }

        // if k == code {
        //     if let Some(m) = m {
        //         let m: Vec<_> = m.iter().map(KeyModifiers::from).collect();
        //         let mut mods = KeyModifiers::NONE;
        //         for m in m {
        //             mods |= m;
        //         }
        //         if modifiers == mods {
        //             //Do something
        //         }
        //     }
        // }

        match code {
            KeyCode::Char(c) => self.handle_char(c, modifiers),
            KeyCode::Down => self.down(),
            KeyCode::Up => self.up(),
            KeyCode::Left => self.browser_prev(),
            KeyCode::Right => self.browser_next(),
            KeyCode::Enter => self.on_enter(),
            KeyCode::Tab => self.on_tab(),
            KeyCode::Backspace => self.search.on_backspace(modifiers),
            KeyCode::Esc => self.on_escape(),
            _ => (),
        }
    }
    pub fn handle_char(&mut self, c: char, modifier: KeyModifiers) {
        if self.app_mode == AppMode::Search {
            self.search.on_key(c);
        } else {
            match c {
                'u' => self.refresh_db(),
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
                'w' => {
                    self.queue.volume_up();
                    self.options.save_volume(self.queue.get_volume());
                }
                's' => {
                    self.queue.volume_down();
                    self.options.save_volume(self.queue.get_volume());
                }
                '/' => self.app_mode = AppMode::Search,
                '.' => self.app_mode = AppMode::Options,
                'x' => self.delete_from_queue(),
                'r' => self.queue.randomize(),
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
    fn refresh_ui(&mut self) {
        self.browser.refresh();
        self.search.refresh();
    }
    fn refresh_db(&mut self) {
        //update all the music in the db
        let paths = self.options.paths();
        paths.iter().for_each(|path| self.db.add(path));
    }
}

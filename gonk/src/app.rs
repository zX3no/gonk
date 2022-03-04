use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use gonk_database::{Bind, Database, Key, Modifier, Toml};
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
                self.search.on_tab();
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
    //TODO: remove bool return
    pub fn input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        let bind = Bind {
            key: Key::from(key),
            modifiers: Modifier::from_u32(modifiers),
        };

        // let hk = Toml::new().unwrap().hotkey;
        let hk = self.options.hotkeys().clone();

        //exit
        if hk.quit.contains(&bind) {
            return true;
        };

        //match the hardcoded cases
        match key {
            KeyCode::Tab => self.on_tab(),
            KeyCode::Backspace => self.search.on_backspace(modifiers),
            KeyCode::Enter => self.on_enter(),
            KeyCode::Esc => self.on_escape(),
            KeyCode::Char('1') | KeyCode::Char('!') => self.queue.move_constraint('1', modifiers),
            KeyCode::Char('2') | KeyCode::Char('@') => self.queue.move_constraint('2', modifiers),
            KeyCode::Char('3') | KeyCode::Char('#') => self.queue.move_constraint('3', modifiers),
            KeyCode::Char(c) => {
                if self.app_mode == AppMode::Search {
                    self.search.on_key(c);
                    return false;
                }
            }
            _ => (),
        }

        match bind {
            _ if hk.up.contains(&bind) => self.up(),
            _ if hk.down.contains(&bind) => self.down(),
            _ if hk.left.contains(&bind) => self.browser_prev(),
            _ if hk.right.contains(&bind) => self.browser_next(),
            _ if hk.play_pause.contains(&bind) => self.queue.play_pause(),
            _ if hk.clear.contains(&bind) => self.queue.clear(),
            _ if hk.refresh_database.contains(&bind) => self.refresh_db(),
            _ if hk.seek_backward.contains(&bind) => self.queue.seek_bw(),
            _ if hk.seek_forward.contains(&bind) => self.queue.seek_fw(),
            _ if hk.previous.contains(&bind) => self.queue.prev(),
            _ if hk.next.contains(&bind) => self.queue.next(),
            _ if hk.volume_up.contains(&bind) => {
                self.queue.volume_up();
                self.options.save_volume(self.queue.get_volume());
            }
            _ if hk.volume_down.contains(&bind) => {
                self.queue.volume_down();
                self.options.save_volume(self.queue.get_volume());
            }
            _ if hk.search.contains(&bind) => self.app_mode = AppMode::Search,
            _ if hk.options.contains(&bind) => self.app_mode = AppMode::Options,
            _ if hk.delete.contains(&bind) => self.delete_from_queue(),
            _ if hk.random.contains(&bind) => self.queue.randomize(),
            _ => (),
        }
        false
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

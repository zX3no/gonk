use crossterm::event::{KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use gronk_database::Database;

pub use {browser::Browser, queue::Queue, search::Search};

pub mod browser;
mod queue;
mod search;

#[derive(PartialEq)]
pub enum AppMode {
    Browser,
    Queue,
    Search,
}

pub struct App<'a> {
    pub db: &'a Database,
    pub browser: Browser<'a>,
    pub queue: Queue,
    pub search: Search<'a>,
    pub ui_mode: AppMode,
}

impl<'a> App<'a> {
    pub fn new(db: &'a Database) -> Self {
        let music = Browser::new(db);
        let queue = Queue::new();
        let search = Search::new(db);

        Self {
            browser: music,
            queue,
            search,
            db,
            ui_mode: AppMode::Browser,
        }
    }
    pub fn browser_next(&mut self) {
        if self.ui_mode == AppMode::Browser {
            self.browser.next();
        }
    }
    pub fn browser_prev(&mut self) {
        if self.ui_mode == AppMode::Browser {
            self.browser.prev();
        }
    }
    pub fn up(&mut self) {
        match self.ui_mode {
            AppMode::Browser => self.browser.up(),
            AppMode::Queue => self.queue.up(),
            AppMode::Search => self.search.up(),
        }
    }
    pub fn down(&mut self) {
        match self.ui_mode {
            AppMode::Browser => self.browser.down(),
            AppMode::Queue => self.queue.down(),
            AppMode::Search => self.search.down(),
        }
    }
    pub fn on_enter(&mut self) {
        match self.ui_mode {
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
        }
    }
    pub fn on_tick(&mut self) {
        self.queue.update();

        if self.search.has_query_changed() {
            self.search.update_search();
        }
    }
    fn delete_from_queue(&mut self) {
        self.queue.delete_selected();
    }
    pub fn reset_db(&mut self) {
        self.db.reset();
        self.browser.refresh();
        self.search.refresh();
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
                    AppMode::Browser => AppMode::Queue,
                    AppMode::Queue => AppMode::Browser,
                    AppMode::Search => {
                        self.search.reset();
                        AppMode::Queue
                    }
                };
            }
            KeyCode::Backspace => self.search.on_backspace(modifiers),
            KeyCode::Esc => {
                if self.ui_mode == AppMode::Search {
                    self.search.exit();
                }
            }
            _ => (),
        }
    }
    pub fn handle_char(&mut self, c: char, modifier: KeyModifiers) {
        if self.ui_mode == AppMode::Search {
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
                '/' => self.ui_mode = AppMode::Search,
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

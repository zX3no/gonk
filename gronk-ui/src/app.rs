use std::io::stdout;

use crate::{browser::Browser, queue::Queue};
use crossterm::{
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};

pub enum Mode {
    Browser,
    Search,
    Queue,
    Seeker,
}

pub struct App {
    pub mode: Mode,
    pub browser: Browser,
    pub queue: Queue,
    pub query: String,
    pub seeker: String,
    pub seeker_ratio: u16,
}

impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();

        Self {
            mode: Mode::Browser,
            browser: Browser::new(),
            queue: Queue::new(),
            query: String::new(),
            seeker: String::from("00:00/00:00"),
            seeker_ratio: 0,
        }
    }
    pub fn on_up(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.up(),
            _ => (),
        }
    }
    pub fn on_down(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.down(),
            _ => (),
        }
    }
    pub fn on_next(&mut self) {
        if let Mode::Search = self.mode {
            self.mode = Mode::Browser;
            self.clear_query();
        } else {
            self.browser.next_mode();
        }
    }
    pub fn on_enter(&mut self) {
        if let Mode::Search = self.mode {
            self.mode = Mode::Browser;
        } else if self.browser.is_song() {
            self.queue.add(self.browser.get_songs());
        } else {
            self.clear_query();
        }
    }
    pub fn on_back(&mut self) {
        if let Mode::Search = self.mode {
            self.query.pop();
            self.browser.refresh();
            self.browser.search(&self.query);
        }
        self.clear_query();
        self.browser.prev_mode();
    }
    pub fn clear_query(&mut self) {
        self.query = String::new();
        self.browser.refresh();
    }
    pub fn on_escape(&mut self) {
        self.mode = Mode::Browser;
        self.browser.refresh();
        self.query = String::new();
    }
    pub fn on_key(&mut self, c: char) {
        if let Mode::Search = self.mode {
            self.query.push(c);
            self.browser.search(&self.query);
            return;
        }
        match c {
            '/' => {
                self.mode = Mode::Search;
            }
            'h' => self.on_back(),
            'j' => self.on_down(),
            'k' => self.on_up(),
            'l' => self.on_next(),
            'q' => self.queue.prev(),
            'e' => self.queue.next(),
            'c' => self.queue.clear(),
            ' ' => self.queue.pause(),
            '-' => self.queue.volume_down(),
            '=' => self.queue.volume_up(),
            _ => (),
        }
    }
    pub fn on_tick(&mut self) {
        // if self.seeker_ratio < 100 {
        //     self.seeker_ratio += 1;
        // } else {
        //     self.seeker_ratio = 0;
        // }
        // self.browser.update();

        //todo broken
        self.seeker = self.queue.get_seeker();
        self.seeker_ratio = self.queue.get_ratio();
    }
}

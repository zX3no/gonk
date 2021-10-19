use std::io::stdout;

use crate::browser::Browser;
use crossterm::{
    event::EnableMouseCapture,
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use gronk_player::player::Player;

pub enum Mode {
    Browser,
    Search,
    Queue,
    Seeker,
}

pub struct App {
    pub mode: Mode,
    pub browser: Browser,
    pub player: Player,
    pub query: String,
    pub seeker: String,
    pub seeker_ratio: u16,
}

impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();

        Self {
            mode: Mode::Browser,
            browser: Browser::new(),
            player: Player::new(),
            query: String::new(),
            seeker: String::from("00:00/00:00"),
            seeker_ratio: 0,
        }
    }
    pub fn run() {}
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
    pub fn on_select(&mut self) {
        if let Mode::Search = self.mode {
            self.mode = Mode::Browser;
            self.clear_query();
        } else {
            self.browser.next_mode();
        }
    }
    pub fn on_back(&mut self) {
        if let Mode::Search = self.mode {
            self.query.pop();
        }
        self.browser.prev_mode();
    }
    pub fn clear_query(&mut self) {
        self.query = String::new();
    }
    pub fn on_escape(&mut self) {
        self.mode = Mode::Browser;
    }
    pub fn on_key(&mut self, c: char) {
        if let Mode::Search = self.mode {
            self.query.push(c);
            return;
        }
        match c {
            '/' => {
                self.mode = Mode::Search;
            }
            'h' => self.on_back(),
            'j' => self.on_down(),
            'k' => self.on_up(),
            'l' => self.on_select(),
            _ => (),
        }
    }
    pub fn on_tick(&mut self) {
        // if self.seeker_ratio < 100 {
        //     self.seeker_ratio += 1;
        // } else {
        //     self.seeker_ratio = 0;
        // }

        self.seeker = self.player.get_seeker();
    }
}

use std::io::stdout;

use crate::browser::Browser;
use crossterm::{
    event::EnableMouseCapture,
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};

pub enum Mode {
    Browser,
    Search,
    Queue,
    Seeker,
}

pub struct App<'a> {
    pub mode: Mode,
    pub browser: Browser<'a>,
    pub query: String,
    pub seeker: String,
    pub seeker_ratio: u16,
}
impl<'a> App<'a> {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();

        Self {
            mode: Mode::Browser,
            browser: Browser::new(),
            query: String::new(),
            seeker: String::from("00:00"),
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
        // if !self.browser.is_song() {
        self.browser.next_mode();
        // }
    }
    pub fn on_back(&mut self) {
        self.browser.prev_mode();
    }

    pub fn on_escape(&mut self) {
        self.mode = Mode::Browser;
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            '/' => {
                self.mode = Mode::Search;
            }
            't' => {
                // self.show_chart = !self.show_chart;
            }
            _ => {}
        }
    }
    pub fn get_seeker(&self) -> String {
        String::from("00:00")
    }
    pub fn on_tick(&mut self) {
        // if self.seeker_ratio < 100 {
        //     self.seeker_ratio += 1;
        // } else {
        //     self.seeker_ratio = 0;
        // }

        //this could be done different?
        // self.seeker = self.get_seeker();
    }
}

use std::io::stdout;

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

pub struct App {
    pub mode: Mode,
    pub seeker: String,
    pub seeker_ratio: u16,
}
impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();

        Self {
            mode: Mode::Browser,
            seeker: String::from("00:00"),
            seeker_ratio: 0,
        }
    }
    pub fn run() {}
    pub fn on_up(&mut self) {
        todo!();
    }

    pub fn on_down(&mut self) {
        todo!();
    }

    pub fn on_right(&mut self) {
        todo!();
    }

    pub fn on_left(&mut self) {
        todo!();
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
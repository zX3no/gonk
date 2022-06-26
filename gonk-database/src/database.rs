use crate::*;
use std::thread::{self, JoinHandle};

#[derive(Debug, Eq, PartialEq)]
pub enum State {
    Busy,
    Idle,
    NeedsUpdate,
}

#[derive(Default)]
pub struct Database {
    handle: Option<JoinHandle<()>>,
}

impl Database {
    pub fn add_path(&mut self, path: &str) {
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                return;
            }
        }

        let path = path.to_string();
        self.handle = Some(thread::spawn(move || {
            add_folder(&path);
        }));
    }

    pub fn refresh(&mut self) {
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                return;
            }
        }

        self.handle = Some(thread::spawn(|| rescan_folders()));
    }

    pub fn state(&mut self) -> State {
        match self.handle {
            Some(ref handle) => {
                let finished = handle.is_finished();
                if finished {
                    self.handle = None;
                    State::NeedsUpdate
                } else {
                    State::Busy
                }
            }
            None => State::Idle,
        }
    }
}

use crate::TOML_DIR;
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tui::style::Color;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Eq)]
pub enum Modifier {
    Control,
    Shift,
    Alt,
}

impl Modifier {
    pub fn from_bitflags(m: KeyModifiers) -> Option<Vec<Self>> {
        match m.bits() {
            0b0000_0001 => Some(vec![Modifier::Shift]),
            0b0000_0100 => Some(vec![Modifier::Alt]),
            0b0000_0010 => Some(vec![Modifier::Control]),
            3 => Some(vec![Modifier::Control, Modifier::Shift]),
            5 => Some(vec![Modifier::Alt, Modifier::Shift]),
            6 => Some(vec![Modifier::Control, Modifier::Alt]),
            7 => Some(vec![Modifier::Control, Modifier::Alt, Modifier::Shift]),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Eq)]
pub struct Key(pub String);

impl From<&str> for Key {
    fn from(key: &str) -> Self {
        Self(key.to_string())
    }
}

impl From<KeyCode> for Key {
    fn from(item: KeyCode) -> Self {
        match item {
            KeyCode::Char(' ') => Key::from("SPACE"),
            KeyCode::Char(c) => Key(c.to_string().to_ascii_uppercase()),
            KeyCode::Backspace => Key::from("BACKSPACE"),
            KeyCode::Enter => Key::from("ENTER"),
            KeyCode::Left => Key::from("LEFT"),
            KeyCode::Right => Key::from("RIGHT"),
            KeyCode::Up => Key::from("UP"),
            KeyCode::Down => Key::from("DOWN"),
            KeyCode::Home => Key::from("HOME"),
            KeyCode::End => Key::from("END"),
            KeyCode::PageUp => Key::from("PAGEUP"),
            KeyCode::PageDown => Key::from("PAGEDOWN"),
            KeyCode::Tab => Key::from("TAB"),
            KeyCode::BackTab => Key::from("BACKTAB"),
            KeyCode::Delete => Key::from("DELETE"),
            KeyCode::Insert => Key::from("INSERT"),
            KeyCode::F(num) => Key(format!("F{num}")),
            KeyCode::Null => Key::from("NULL"),
            KeyCode::Esc => Key::from("ESCAPE"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Eq)]
pub struct Bind {
    pub key: Key,
    pub modifiers: Option<Vec<Modifier>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Hotkey {
    pub up: Bind,
    pub down: Bind,
    pub left: Bind,
    pub right: Bind,
    pub play_pause: Bind,
    pub volume_up: Bind,
    pub volume_down: Bind,
    pub next: Bind,
    pub previous: Bind,
    pub seek_forward: Bind,
    pub seek_backward: Bind,
    pub clear: Bind,
    pub clear_except_playing: Bind,
    pub delete: Bind,
    pub random: Bind,
    pub refresh_database: Bind,
    pub quit: Bind,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub paths: Vec<String>,
    pub output_device: String,
    pub volume: u16,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Colors {
    pub number: Color,
    pub name: Color,
    pub album: Color,
    pub artist: Color,
    pub seeker: Color,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Toml {
    pub config: Config,
    pub colors: Colors,
    pub hotkey: Hotkey,
}

impl Toml {
    pub fn new() -> Self {
        let file = if TOML_DIR.exists() {
            fs::read_to_string(TOML_DIR.as_path()).unwrap()
        } else {
            let toml = Toml {
                config: Config {
                    paths: Vec::new(),
                    output_device: String::new(),
                    volume: 15,
                },
                colors: Colors {
                    number: Color::Green,
                    name: Color::Cyan,
                    album: Color::Magenta,
                    artist: Color::Blue,
                    seeker: Color::White,
                },
                hotkey: Hotkey {
                    up: Bind {
                        key: Key::from("K"),
                        modifiers: None,
                    },
                    down: Bind {
                        key: Key::from("J"),
                        modifiers: None,
                    },
                    left: Bind {
                        key: Key::from("H"),
                        modifiers: None,
                    },
                    right: Bind {
                        key: Key::from("L"),
                        modifiers: None,
                    },
                    play_pause: Bind {
                        key: Key::from("SPACE"),
                        modifiers: None,
                    },
                    volume_up: Bind {
                        key: Key::from("W"),
                        modifiers: None,
                    },
                    volume_down: Bind {
                        key: Key::from("S"),
                        modifiers: None,
                    },
                    seek_forward: Bind {
                        key: Key::from("E"),
                        modifiers: None,
                    },
                    seek_backward: Bind {
                        key: Key::from("Q"),
                        modifiers: None,
                    },
                    next: Bind {
                        key: Key::from("D"),
                        modifiers: None,
                    },
                    previous: Bind {
                        key: Key::from("A"),
                        modifiers: None,
                    },
                    clear: Bind {
                        key: Key::from("C"),
                        modifiers: None,
                    },
                    clear_except_playing: Bind {
                        key: Key::from("C"),
                        modifiers: Some(vec![Modifier::Shift]),
                    },
                    delete: Bind {
                        key: Key::from("X"),
                        modifiers: None,
                    },
                    random: Bind {
                        key: Key::from("R"),
                        modifiers: None,
                    },
                    refresh_database: Bind {
                        key: Key::from("U"),
                        modifiers: None,
                    },
                    quit: Bind {
                        key: Key::from("C"),
                        modifiers: Some(vec![Modifier::Control]),
                    },
                },
            };

            match toml::to_string_pretty(&toml) {
                Ok(toml) => toml,
                Err(err) => panic!("{}", &err),
            }
        };

        match toml::from_str(&file) {
            Ok(toml) => toml,
            Err(err) => {
                //TODO: parse and describe error to user?
                panic!("{:#?}", &err);
            }
        }
    }
    pub fn check_paths(self) -> Result<Self, String> {
        for path in &self.config.paths {
            let path = Path::new(&path);
            if !path.exists() {
                return Err(format!("{} is not a valid path.", path.to_string_lossy()));
            }
        }
        Ok(self)
    }
    pub fn volume(&self) -> u16 {
        self.config.volume
    }
    pub fn paths(&self) -> &[String] {
        &self.config.paths
    }
    pub fn output_device(&self) -> &str {
        &self.config.output_device
    }
    pub fn add_path(&mut self, path: String) {
        if !self.config.paths.contains(&path) {
            self.config.paths.push(path);
            self.write();
        }
    }
    pub fn set_volume(&mut self, vol: u16) {
        self.config.volume = vol;
        self.write();
    }
    pub fn set_output_device(&mut self, device: String) {
        self.config.output_device = device;
        self.write();
    }
    pub fn write(&self) {
        let toml = toml::to_string(&self).expect("Failed to write toml file.");
        fs::write(TOML_DIR.as_path(), toml).expect("Could not write toml flie.");
    }
    pub fn reset(&mut self) {
        self.config.paths = Vec::new();
        self.write();
    }
}

impl Default for Toml {
    fn default() -> Self {
        Toml::new()
    }
}

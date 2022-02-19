use crate::TOML_DIR;
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::fs;
use tui::style::Color;

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Clone)]
pub enum Modifier {
    CONTROL,
    SHIFT,
    ALT,
}

impl From<&Modifier> for KeyModifiers {
    fn from(m: &Modifier) -> Self {
        match m {
            Modifier::CONTROL => KeyModifiers::CONTROL,
            Modifier::SHIFT => KeyModifiers::SHIFT,
            Modifier::ALT => KeyModifiers::ALT,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Key(pub String);
impl From<&str> for Key {
    fn from(key: &str) -> Self {
        Self(key.to_string())
    }
}

impl From<KeyCode> for Key {
    fn from(item: KeyCode) -> Self {
        match item {
            KeyCode::Char(c) => Key(c.to_string()),
            _ => Key::from(""),
        }
    }
}

impl From<Key> for KeyCode {
    fn from(val: Key) -> Self {
        match val.0.as_str() {
            "SPACE" => KeyCode::Char(' '),
            "TAB" => KeyCode::Tab,
            _ => {
                let mut chars = val.0.chars();
                KeyCode::Char(chars.next().unwrap())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
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
    pub delete: Bind,
    pub search: Bind,
    pub options: Bind,
    pub change_mode: Bind,
    pub quit: Bind,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    paths: Vec<String>,
    output_device: String,
    volume: u16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Colors {
    pub track: Color,
    pub title: Color,
    pub album: Color,
    pub artist: Color,
}

#[derive(Serialize, Deserialize)]
pub struct Toml {
    pub config: Config,
    pub colors: Colors,
    pub hotkey: Hotkey,
}

impl Toml {
    pub fn new() -> std::io::Result<Self> {
        let path = TOML_DIR.as_path();

        let file = if path.exists() {
            fs::read_to_string(path)?
        } else {
            let toml = Toml {
                config: Config {
                    paths: Vec::new(),
                    output_device: String::new(),
                    volume: 15,
                },
                colors: Colors {
                    track: Color::Green,
                    title: Color::Cyan,
                    album: Color::Magenta,
                    artist: Color::Blue,
                },
                hotkey: Hotkey {
                    up: Bind {
                        key: Key::from("k"),
                        modifiers: None,
                    },
                    down: Bind {
                        key: Key::from("j"),
                        modifiers: None,
                    },
                    left: Bind {
                        key: Key::from("h"),
                        modifiers: None,
                    },
                    right: Bind {
                        key: Key::from("l"),
                        modifiers: None,
                    },
                    play_pause: Bind {
                        key: Key::from("SPACE"),
                        modifiers: None,
                    },
                    volume_up: Bind {
                        key: Key::from("w"),
                        modifiers: None,
                    },
                    volume_down: Bind {
                        key: Key::from("s"),
                        modifiers: None,
                    },
                    seek_forward: Bind {
                        key: Key::from("q"),
                        modifiers: None,
                    },
                    seek_backward: Bind {
                        key: Key::from("e"),
                        modifiers: None,
                    },
                    next: Bind {
                        key: Key::from("a"),
                        modifiers: None,
                    },
                    previous: Bind {
                        key: Key::from("d"),
                        modifiers: None,
                    },
                    clear: Bind {
                        key: Key::from("c"),
                        modifiers: None,
                    },
                    delete: Bind {
                        key: Key::from("x"),
                        modifiers: None,
                    },
                    search: Bind {
                        key: Key::from("/"),
                        modifiers: None,
                    },
                    options: Bind {
                        key: Key::from("."),
                        modifiers: None,
                    },
                    change_mode: Bind {
                        key: Key::from("TAB"),
                        modifiers: None,
                    },
                    quit: Bind {
                        key: Key::from("c"),
                        modifiers: Some(vec![Modifier::CONTROL]),
                    },
                },
            };

            match toml::to_string_pretty(&toml) {
                Ok(toml) => toml,
                Err(err) => panic!("{}", &err),
            }
        };

        match toml::from_str(&file) {
            Ok(toml) => Ok(toml),
            Err(err) => {
                //TODO: parse and describe error to user?
                panic!("{:#?}", &err);
            }
        }
    }
    pub fn volume(&self) -> u16 {
        self.config.volume
    }
    pub fn paths(&self) -> Vec<String> {
        self.config.paths.clone()
    }
    pub fn output_device(&self) -> String {
        self.config.output_device.clone()
    }
    pub fn add_path(&mut self, path: String) {
        if !self.config.paths.contains(&path) {
            self.config.paths.push(path);
            self.write();
        }
    }
    pub fn remove_path(&mut self, path: &str) {
        self.config.paths.retain(|x| x != path);
        self.write();
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
        let toml = toml::to_string(&self).unwrap();
        fs::write(TOML_DIR.as_path(), toml).unwrap();
    }
}

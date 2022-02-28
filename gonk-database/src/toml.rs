use crate::TOML_DIR;
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::fs;
use tui::style::Color;
use win_hotkey::{keys, modifiers};

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Modifier {
    CONTROL,
    SHIFT,
    ALT,
}
impl Modifier {
    pub fn as_u32(&self) -> u32 {
        match self {
            Modifier::CONTROL => modifiers::CONTROL,
            Modifier::SHIFT => modifiers::SHIFT,
            Modifier::ALT => modifiers::ALT,
        }
    }
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Bind {
    pub keys: Vec<Key>,
    pub modifiers: Option<Vec<Modifier>>,
}

impl Bind {
    pub fn modifiers(&self) -> u32 {
        if let Some(m) = &self.modifiers {
            m.iter().map(|m| m.as_u32()).sum()
        } else {
            0
        }
    }
    pub fn key(&self) -> u32 {
        if let Some(key) = self.keys.first() {
            match key.0.as_str() {
                "SPACE" => keys::SPACEBAR,
                "BACKSPACE" => keys::BACKSPACE,
                "ENTER" => keys::ENTER,
                "UP" => keys::ARROW_UP,
                "DOWN" => keys::ARROW_DOWN,
                "LEFT" => keys::ARROW_LEFT,
                "RIGHT" => keys::ARROW_RIGHT,
                "HOME" => keys::HOME,
                "END" => keys::END,
                "PAGEUP" => keys::PAGE_UP,
                "PAGEDOWN" => keys::PAGE_DOWN,
                "TAB" => keys::TAB,
                "DELETE" => keys::DELETE,
                "INSERT" => keys::INSERT,
                "ESCAPE" => keys::ESCAPE,
                "CAPSLOCK" => keys::CAPS_LOCK,
                _ => 0,
            }
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub struct SimpleBind {
    pub key: Key,
    pub modifiers: KeyModifiers,
}

impl PartialEq<Bind> for SimpleBind {
    fn eq(&self, other: &Bind) -> bool {
        let m = if let Some(modifiers) = &other.modifiers {
            let m: Vec<_> = modifiers.iter().map(KeyModifiers::from).collect();
            let mut mods = KeyModifiers::NONE;
            for m in m {
                mods |= m;
            }
            mods
        } else {
            KeyModifiers::NONE
        };

        //check if one or more of the keys match
        other.keys.contains(&self.key) && self.modifiers == m
    }
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
    pub random: Bind,
    pub change_mode: Bind,
    pub refresh_database: Bind,
    pub quit: Bind,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalHotkey {
    pub play_pause: Bind,
    pub volume_up: Bind,
    pub volume_down: Bind,
    pub next: Bind,
    pub previous: Bind,
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
    pub global_hotkey: GlobalHotkey,
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
                global_hotkey: GlobalHotkey {
                    play_pause: Bind {
                        keys: vec![Key::from("CAPSLOCK")],
                        modifiers: Some(vec![Modifier::SHIFT]),
                    },
                    volume_up: Bind {
                        keys: vec![Key::from("2")],
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                    volume_down: Bind {
                        keys: vec![Key::from("1")],
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                    next: Bind {
                        keys: vec![Key::from("W")],
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                    previous: Bind {
                        keys: vec![Key::from("Q")],
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                },
                hotkey: Hotkey {
                    up: Bind {
                        keys: vec![Key::from("K"), Key::from("UP")],
                        modifiers: None,
                    },
                    down: Bind {
                        keys: vec![Key::from("J"), Key::from("DOWN")],
                        modifiers: None,
                    },
                    left: Bind {
                        keys: vec![Key::from("H"), Key::from("LEFT")],
                        modifiers: None,
                    },
                    right: Bind {
                        keys: vec![Key::from("L"), Key::from("RIGHT")],
                        modifiers: None,
                    },
                    play_pause: Bind {
                        keys: vec![Key::from("SPACE")],
                        modifiers: None,
                    },
                    volume_up: Bind {
                        keys: vec![Key::from("W")],
                        modifiers: None,
                    },
                    volume_down: Bind {
                        keys: vec![Key::from("S")],
                        modifiers: None,
                    },
                    seek_forward: Bind {
                        keys: vec![Key::from("E")],
                        modifiers: None,
                    },
                    seek_backward: Bind {
                        keys: vec![Key::from("Q")],
                        modifiers: None,
                    },
                    next: Bind {
                        keys: vec![Key::from("D")],
                        modifiers: None,
                    },
                    previous: Bind {
                        keys: vec![Key::from("A")],
                        modifiers: None,
                    },
                    clear: Bind {
                        keys: vec![Key::from("C")],
                        modifiers: None,
                    },
                    delete: Bind {
                        keys: vec![Key::from("X")],
                        modifiers: None,
                    },
                    search: Bind {
                        keys: vec![Key::from("/")],
                        modifiers: None,
                    },
                    options: Bind {
                        keys: vec![Key::from(".")],
                        modifiers: None,
                    },
                    random: Bind {
                        keys: vec![Key::from("R")],
                        modifiers: None,
                    },
                    change_mode: Bind {
                        keys: vec![Key::from("TAB")],
                        modifiers: None,
                    },
                    refresh_database: Bind {
                        keys: vec![Key::from("U")],
                        modifiers: None,
                    },
                    quit: Bind {
                        keys: vec![Key::from("C")],
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

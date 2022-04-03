use crate::TOML_DIR;
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::fs;
use tui::style::Color;

//TODO: test on linux
#[cfg(windows)]
use win_hotkey::{keys, modifiers};

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Modifier {
    CONTROL,
    SHIFT,
    ALT,
}

impl Modifier {
    #[cfg(windows)]
    pub fn as_u32(&self) -> u32 {
        match self {
            Modifier::CONTROL => modifiers::CONTROL,
            Modifier::SHIFT => modifiers::SHIFT,
            Modifier::ALT => modifiers::ALT,
        }
    }
    pub fn from_u32(m: KeyModifiers) -> Option<Vec<Self>> {
        //TODO: this doesn't support triple modfifier combos
        //plus this is stupid, surely there is a better way
        match m.bits() {
            0b0000_0001 => Some(vec![Modifier::SHIFT]),
            0b0000_0100 => Some(vec![Modifier::ALT]),
            0b0000_0010 => Some(vec![Modifier::CONTROL]),
            3 => Some(vec![Modifier::CONTROL, Modifier::SHIFT]),
            5 => Some(vec![Modifier::ALT, Modifier::SHIFT]),
            6 => Some(vec![Modifier::CONTROL, Modifier::ALT]),
            _ => None,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Bind {
    pub key: Key,
    pub modifiers: Option<Vec<Modifier>>,
}

impl Bind {
    pub fn new(key: &str) -> Self {
        Self {
            key: Key::from(key),
            modifiers: None,
        }
    }

    #[cfg(windows)]
    pub fn modifiers(&self) -> u32 {
        if let Some(m) = &self.modifiers {
            m.iter().map(Modifier::as_u32).sum()
        } else {
            0
        }
    }

    #[cfg(windows)]
    pub fn key(&self) -> u32 {
        match self.key.0.as_str() {
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
            key => {
                if let Some(char) = key.chars().next() {
                    char as u32
                } else {
                    0
                }
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn test() {
    let b = Bind {
        key: Key::from("A"),
        modifiers: Some(vec![Modifier::ALT, Modifier::SHIFT]),
    };
    assert_eq!(Bind::new("A").key(), 'A' as u32);
    assert_eq!(Bind::new("TAB").key(), keys::TAB);
    assert_eq!(b.modifiers(), modifiers::ALT | modifiers::SHIFT);
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Hotkey {
    pub up: Vec<Bind>,
    pub down: Vec<Bind>,
    pub left: Vec<Bind>,
    pub right: Vec<Bind>,
    pub play_pause: Vec<Bind>,
    pub volume_up: Vec<Bind>,
    pub volume_down: Vec<Bind>,
    pub next: Vec<Bind>,
    pub previous: Vec<Bind>,
    pub seek_forward: Vec<Bind>,
    pub seek_backward: Vec<Bind>,
    pub clear: Vec<Bind>,
    pub delete: Vec<Bind>,
    pub search: Vec<Bind>,
    pub options: Vec<Bind>,
    pub random: Vec<Bind>,
    pub change_mode: Vec<Bind>,
    pub refresh_database: Vec<Bind>,
    pub quit: Vec<Bind>,
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
                        key: Key::from("CAPSLOCK"),
                        modifiers: Some(vec![Modifier::SHIFT]),
                    },
                    volume_up: Bind {
                        key: Key::from("2"),
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                    volume_down: Bind {
                        key: Key::from("1"),
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                    next: Bind {
                        key: Key::from("W"),
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                    previous: Bind {
                        key: Key::from("Q"),
                        modifiers: Some(vec![Modifier::SHIFT, Modifier::ALT]),
                    },
                },
                hotkey: Hotkey {
                    up: vec![
                        Bind {
                            key: Key::from("K"),
                            modifiers: None,
                        },
                        Bind {
                            key: Key::from("UP"),
                            modifiers: None,
                        },
                    ],
                    down: vec![
                        Bind {
                            key: Key::from("J"),
                            modifiers: None,
                        },
                        Bind {
                            key: Key::from("DOWN"),
                            modifiers: None,
                        },
                    ],
                    left: vec![
                        Bind {
                            key: Key::from("H"),
                            modifiers: None,
                        },
                        Bind {
                            key: Key::from("LEFT"),
                            modifiers: None,
                        },
                    ],
                    right: vec![
                        Bind {
                            key: Key::from("L"),
                            modifiers: None,
                        },
                        Bind {
                            key: Key::from("RIGHT"),
                            modifiers: None,
                        },
                    ],
                    play_pause: vec![Bind {
                        key: Key::from("SPACE"),
                        modifiers: None,
                    }],
                    volume_up: vec![Bind {
                        key: Key::from("W"),
                        modifiers: None,
                    }],
                    volume_down: vec![Bind {
                        key: Key::from("S"),
                        modifiers: None,
                    }],
                    seek_forward: vec![Bind {
                        key: Key::from("E"),
                        modifiers: None,
                    }],
                    seek_backward: vec![Bind {
                        key: Key::from("Q"),
                        modifiers: None,
                    }],
                    next: vec![Bind {
                        key: Key::from("D"),
                        modifiers: None,
                    }],
                    previous: vec![Bind {
                        key: Key::from("A"),
                        modifiers: None,
                    }],
                    clear: vec![Bind {
                        key: Key::from("C"),
                        modifiers: None,
                    }],
                    delete: vec![Bind {
                        key: Key::from("X"),
                        modifiers: None,
                    }],
                    search: vec![Bind {
                        key: Key::from("/"),
                        modifiers: None,
                    }],
                    options: vec![Bind {
                        key: Key::from("."),
                        modifiers: None,
                    }],
                    random: vec![Bind {
                        key: Key::from("R"),
                        modifiers: None,
                    }],
                    change_mode: vec![Bind {
                        key: Key::from("TAB"),
                        modifiers: None,
                    }],
                    refresh_database: vec![Bind {
                        key: Key::from("U"),
                        modifiers: None,
                    }],
                    quit: vec![Bind {
                        key: Key::from("C"),
                        modifiers: Some(vec![Modifier::CONTROL]),
                    }],
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
    //TODO: paths are not updated in real time
    pub fn paths(&self) -> &Vec<String> {
        &self.config.paths
    }
    pub fn output_device(&self) -> &String {
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
}

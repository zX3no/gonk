use crate::keycodes::{Bind, Key, Modifier};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tui::style::Color;

pub struct Config<T> {
    pub data: T,
    pub path: PathBuf,
}

impl<T> Config<T>
where
    T: DeserializeOwned + Serialize + Default,
{
    pub fn new(path: &Path) -> Self {
        if path.exists() {
            let file = fs::read_to_string(path).unwrap();
            match toml::from_str(&file) {
                Ok(data) => Self {
                    data,
                    path: path.to_path_buf(),
                },
                Err(err) => {
                    //TODO: parse and describe error to user?
                    panic!("{:#?}", &err);
                }
            }
        } else {
            let config = Self {
                data: T::default(),
                path: path.to_path_buf(),
            };
            config.write();
            config
        }
    }
    pub fn write(&self) {
        fs::write(&self.path, toml::to_string(&self.data).unwrap()).unwrap();
    }
}

#[derive(Serialize, Deserialize)]
pub struct Server {
    pub paths: Vec<String>,
    pub ip: String,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            ip: String::from("localhost:673"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GlobalHotkey {
    pub play_pause: Bind,
    pub volume_up: Bind,
    pub volume_down: Bind,
    pub next: Bind,
    pub previous: Bind,
    pub quit: Bind,
}

impl Default for GlobalHotkey {
    fn default() -> Self {
        Self {
            play_pause: Bind {
                key: Key::from("ESCAPE"),
                modifiers: Some(vec![Modifier::Shift]),
            },
            volume_up: Bind {
                key: Key::from("2"),
                modifiers: Some(vec![Modifier::Shift, Modifier::Alt]),
            },
            volume_down: Bind {
                key: Key::from("1"),
                modifiers: Some(vec![Modifier::Shift, Modifier::Alt]),
            },
            next: Bind {
                key: Key::from("W"),
                modifiers: Some(vec![Modifier::Shift, Modifier::Alt]),
            },
            previous: Bind {
                key: Key::from("Q"),
                modifiers: Some(vec![Modifier::Shift, Modifier::Alt]),
            },
            quit: Bind {
                key: Key::from("Z"),
                modifiers: Some(vec![Modifier::Shift, Modifier::Control, Modifier::Alt]),
            },
        }
    }
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
    pub refresh: Vec<Bind>,
    pub quit: Vec<Bind>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Colors {
    pub number: Color,
    pub name: Color,
    pub album: Color,
    pub artist: Color,
}

#[derive(Serialize, Deserialize)]
pub struct Client {
    pub server_ip: String,
    pub colors: Colors,
    pub hotkey: Hotkey,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            server_ip: String::from("localhost:673"),
            colors: Colors {
                number: Color::Green,
                name: Color::Cyan,
                album: Color::Magenta,
                artist: Color::Blue,
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
                refresh: vec![Bind {
                    key: Key::from("U"),
                    modifiers: None,
                }],
                quit: vec![Bind {
                    key: Key::from("C"),
                    modifiers: Some(vec![Modifier::Control]),
                }],
            },
        }
    }
}

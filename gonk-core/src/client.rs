use crate::{Bind, Key, Modifier, CLIENT_CONFIG};
use serde::{Deserialize, Serialize};
use std::fs;
use tui::style::Color;

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
pub struct Colors {
    pub number: Color,
    pub name: Color,
    pub album: Color,
    pub artist: Color,
}

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    pub colors: Colors,
    pub hotkey: Hotkey,
}

impl ClientConfig {
    pub fn new() -> Self {
        if CLIENT_CONFIG.exists() {
            let file = fs::read_to_string(CLIENT_CONFIG.as_path()).unwrap();
            match toml::from_str(&file) {
                Ok(toml) => toml,
                Err(err) => {
                    //TODO: parse and describe error to user?
                    panic!("{:#?}", &err);
                }
            }
        } else {
            let toml = ClientConfig {
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
                    refresh_database: vec![Bind {
                        key: Key::from("U"),
                        modifiers: None,
                    }],
                    quit: vec![Bind {
                        key: Key::from("C"),
                        modifiers: Some(vec![Modifier::Control]),
                    }],
                },
            };

            fs::write(CLIENT_CONFIG.as_path(), toml::to_string(&toml).unwrap()).unwrap();
            toml
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig::new()
    }
}

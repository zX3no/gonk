use crate::TOML_DIR;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Bind {
    key: String,
    modifier: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Hotkey {
    up: Bind,
    down: Bind,
    left: Bind,
    right: Bind,
    play_pause: Bind,
    volume_up: Bind,
    volume_down: Bind,
    next: Bind,
    previous: Bind,
    seek_forward: Bind,
    seek_backward: Bind,
    clear: Bind,
    delete: Bind,
    search: Bind,
    options: Bind,
    change_mode: Bind,
    quit: Bind,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    paths: Vec<String>,
    output_device: String,
    volume: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Color {
    Green,
    Cyan,
    Blue,
    Magenta,
    White,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Colors {
    track: Color,
    title: Color,
    album: Color,
    artist: Color,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Toml {
    config: Config,
    colors: Colors,
    hotkey: Hotkey,
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
                        key: String::from("k"),
                        modifier: None,
                    },
                    down: Bind {
                        key: String::from("j"),
                        modifier: None,
                    },
                    left: Bind {
                        key: String::from("h"),
                        modifier: None,
                    },
                    right: Bind {
                        key: String::from("l"),
                        modifier: None,
                    },
                    play_pause: Bind {
                        key: String::from("space"),
                        modifier: None,
                    },
                    volume_up: Bind {
                        key: String::from("w"),
                        modifier: None,
                    },
                    volume_down: Bind {
                        key: String::from("s"),
                        modifier: None,
                    },
                    seek_forward: Bind {
                        key: String::from("q"),
                        modifier: None,
                    },
                    seek_backward: Bind {
                        key: String::from("e"),
                        modifier: None,
                    },
                    next: Bind {
                        key: String::from("a"),
                        modifier: None,
                    },
                    previous: Bind {
                        key: String::from("d"),
                        modifier: None,
                    },
                    clear: Bind {
                        key: String::from("c"),
                        modifier: None,
                    },
                    delete: Bind {
                        key: String::from("x"),
                        modifier: None,
                    },
                    search: Bind {
                        key: String::from("/"),
                        modifier: None,
                    },
                    options: Bind {
                        key: String::from("."),
                        modifier: None,
                    },
                    change_mode: Bind {
                        key: String::from("tab"),
                        modifier: None,
                    },
                    quit: Bind {
                        key: String::from("c"),
                        modifier: Some(vec![String::from("CONTROL")]),
                    },
                },
            };

            toml::to_string(&toml).unwrap()
        };

        let toml: Toml = toml::from_str(&file)?;

        Ok(toml)
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

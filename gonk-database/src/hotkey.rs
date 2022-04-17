use crate::{Bind, Key, Modifier, HOTKEY_CONFIG};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub play_pause: Bind,
    pub volume_up: Bind,
    pub volume_down: Bind,
    pub next: Bind,
    pub previous: Bind,
}

impl HotkeyConfig {
    pub fn new() -> Self {
        optick::event!("new toml");

        if HOTKEY_CONFIG.exists() {
            let file = fs::read_to_string(HOTKEY_CONFIG.as_path()).unwrap();
            match toml::from_str(&file) {
                Ok(toml) => toml,
                Err(err) => {
                    //TODO: parse and describe error to user?
                    panic!("{:#?}", &err);
                }
            }
        } else {
            HotkeyConfig {
                play_pause: Bind {
                    key: Key::from("CAPSLOCK"),
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
            }
        }
    }
    pub fn write(&self) {
        fs::write(HOTKEY_CONFIG.as_path(), toml::to_string(&self).unwrap()).unwrap();
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self::new()
    }
}
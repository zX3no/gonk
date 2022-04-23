pub use crate::{
    client::{ClientConfig, Colors},
    hotkey::HotkeyConfig,
    index::Index,
    server::ServerConfig,
    song::Song,
    sqlite::Database,
};

use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use static_init::dynamic;
use std::{env, fs, path::PathBuf};

#[cfg(windows)]
use win_hotkey::{keys, modifiers};

mod client;
mod hotkey;
mod index;
mod server;
mod song;
mod sqlite;

#[dynamic]
static GONK_DIR: PathBuf = {
    let config = {
        if let Ok(home) = env::var("HOME") {
            PathBuf::from(home)
        } else if let Ok(home) = env::var("APPDATA") {
            PathBuf::from(home)
        } else if let Ok(home) = env::var("XDG_HOME") {
            PathBuf::from(home)
        } else {
            panic!("HOME, XDG_HOME and APPDATA enviroment variables are all empty?");
        }
    };
    let gonk = config.join("gonk");
    if !config.exists() {
        fs::create_dir(&config).unwrap();
    }
    if !gonk.exists() {
        fs::create_dir(&gonk).unwrap();
    }
    gonk
};

#[dynamic]
pub static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
pub static CLIENT_CONFIG: PathBuf = GONK_DIR.join("gonk.toml");

#[dynamic]
pub static SERVER_CONFIG: PathBuf = GONK_DIR.join("server.toml");

#[dynamic]
pub static HOTKEY_CONFIG: PathBuf = GONK_DIR.join("hotkeys.toml");

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Modifier {
    Control,
    Shift,
    Alt,
}

impl Modifier {
    #[cfg(windows)]
    pub fn as_u32(&self) -> u32 {
        match self {
            Modifier::Control => modifiers::CONTROL,
            Modifier::Shift => modifiers::SHIFT,
            Modifier::Alt => modifiers::ALT,
        }
    }
    pub fn from_u32(m: KeyModifiers) -> Option<Vec<Self>> {
        //TODO: this doesn't support triple modfifier combos
        //plus this is stupid, surely there is a better way
        match m.bits() {
            0b0000_0001 => Some(vec![Modifier::Shift]),
            0b0000_0100 => Some(vec![Modifier::Alt]),
            0b0000_0010 => Some(vec![Modifier::Control]),
            3 => Some(vec![Modifier::Control, Modifier::Shift]),
            5 => Some(vec![Modifier::Alt, Modifier::Shift]),
            6 => Some(vec![Modifier::Control, Modifier::Alt]),
            _ => None,
        }
    }
}

impl From<&Modifier> for KeyModifiers {
    fn from(m: &Modifier) -> Self {
        match m {
            Modifier::Control => KeyModifiers::CONTROL,
            Modifier::Shift => KeyModifiers::SHIFT,
            Modifier::Alt => KeyModifiers::ALT,
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

    #[cfg(windows)]
    pub fn modifiers(&self) -> u32 {
        if let Some(m) = &self.modifiers {
            m.iter().map(Modifier::as_u32).sum()
        } else {
            0
        }
    }

    #[cfg(unix)]
    pub fn key(&self) -> Key {
        self.key
    }

    #[cfg(unix)]
    pub fn modifiers(&self) -> Option<Vec<Modifier>> {
        self.modifiers
    }
}

#[cfg(windows)]
#[test]
fn test() {
    let b = Bind {
        key: Key::from("A"),
        modifiers: Some(vec![Modifier::Alt, Modifier::Shift]),
    };
    assert_eq!(Bind::new("A").key(), 'A' as u32);
    assert_eq!(Bind::new("TAB").key(), keys::TAB);
    assert_eq!(b.modifiers(), modifiers::ALT | modifiers::SHIFT);
}

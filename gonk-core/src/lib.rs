pub use crate::{config::*, index::Index, keycodes::*, song::Song, sqlite::Database};

use static_init::dynamic;
use std::{env, fs, path::PathBuf};

mod config;
mod index;
mod keycodes;
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

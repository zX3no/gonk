pub use crate::{
    index::Index,
    song::Song,
    sqlite::Database,
    toml::{Bind, Colors, GlobalHotkey, Hotkey, Key, Modifier, Toml},
};
use static_init::dynamic;
use std::path::PathBuf;

mod index;
mod song;
mod sqlite;
mod toml;

#[dynamic]
pub static GONK_DIR: PathBuf = {
    let gonk = dirs::config_dir().unwrap().join("gonk");
    if !gonk.exists() {
        std::fs::create_dir_all(GONK_DIR.as_path()).unwrap();
    }
    gonk
};

#[dynamic]
pub static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
pub static TOML_DIR: PathBuf = GONK_DIR.join("gonk.toml");

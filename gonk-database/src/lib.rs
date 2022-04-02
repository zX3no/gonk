pub use crate::{
    sqlite::Database,
    toml::{Bind, Colors, Hotkey, Key, Modifier, Toml},
};
use static_init::dynamic;
use std::path::PathBuf;

mod sqlite;
mod toml;

#[dynamic]
static CONFIG_DIR: PathBuf = dirs::config_dir().unwrap();

#[dynamic]
pub static GONK_DIR: PathBuf = CONFIG_DIR.join("gonk");

#[dynamic]
pub static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
pub static TOML_DIR: PathBuf = GONK_DIR.join("gonk.toml");

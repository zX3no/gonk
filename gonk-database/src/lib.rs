pub use crate::{
    sqlite::Database,
    toml::{Bind, Colors, Hotkey, Key, Modifier, Toml},
};
use static_init::dynamic;
use std::path::PathBuf;

mod sqlite;
mod toml;

#[dynamic]
static DIR: PathBuf = dirs::config_dir().unwrap();

#[dynamic]
pub static CONFIG_DIR: PathBuf = DIR.join("gonk");

#[dynamic]
pub static DB_DIR: PathBuf = DIR.join("gonk\\gonk.db");

#[dynamic]
pub static TOML_DIR: PathBuf = DIR.join("gonk\\gonk.toml");

pub use crate::{
    client::{Bind, ClientConfig, Colors, GlobalHotkey, Hotkey, Key, Modifier},
    server::ServerConfig,
    sqlite::Database,
};
use static_init::dynamic;
use std::path::PathBuf;

mod client;
mod server;
mod sqlite;

#[dynamic]
static CONFIG_DIR: PathBuf = dirs::config_dir().unwrap();

#[dynamic]
pub static GONK_DIR: PathBuf = CONFIG_DIR.join("gonk");

#[dynamic]
pub static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
pub static CLIENT_CONFIG: PathBuf = GONK_DIR.join("gonk.toml");

#[dynamic]
pub static SERVER_CONFIG: PathBuf = GONK_DIR.join("gonk-server.toml");

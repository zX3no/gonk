pub use crate::{sqlite::Database, toml::Toml};
use std::path::PathBuf;

mod sqlite;
mod toml;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref CONFIG_DIR: PathBuf = {
        let config_dir = dirs::config_dir().unwrap();
        config_dir.join("gonk")
    };
    static ref DB_DIR: PathBuf = {
        let db_dir = dirs::config_dir().unwrap();
        db_dir.join("gonk\\gonk.db")
    };
    static ref TOML_DIR: PathBuf = {
        let db_dir = dirs::config_dir().unwrap();
        db_dir.join("gonk\\gonk.toml")
    };
}

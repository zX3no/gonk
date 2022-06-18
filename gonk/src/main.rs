use crate::toml::{Colors, Toml};
use app::App;
use static_init::dynamic;
use std::{
    io::{Result, Stdout},
    path::PathBuf,
};
use tui::backend::CrosstermBackend;

mod app;
mod sqlite;
mod toml;
mod widgets;

#[dynamic]
static GONK_DIR: PathBuf = {
    let gonk = if cfg!(windows) {
        PathBuf::from(&std::env::var("APPDATA").unwrap())
    } else {
        PathBuf::from(&std::env::var("HOME").unwrap()).join(".config")
    }
    .join("gonk");

    if !gonk.exists() {
        std::fs::create_dir_all(&gonk).unwrap();
    }
    gonk
};

#[dynamic]
static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
static TOML_DIR: PathBuf = GONK_DIR.join("gonk.toml");

#[dynamic]
static COLORS: Colors = Toml::new().colors;

type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    sqlite::initialize_database();

    match App::new() {
        Ok(mut app) => app.run(),
        Err(err) => Ok(println!("{}", err)),
    }
}

mod database;
use std::{fs::File, io::Write, path::PathBuf};

use database::Database;
use hashbrown::HashMap;

use crate::database::{Album, Artist, Song};

fn main() {
    let path = r"D:\OneDrive\Music\Badbadnotgood";
    // let database = Database::create(r"D:\OneDrive\Music\Badbadnotgood");
    Database::create_map(path);

    // let mut file = File::create("music.toml").unwrap();

    // let output = toml::to_string(&database).unwrap();
    // file.write_all(output.as_bytes()).unwrap();
}

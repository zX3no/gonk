#[macro_use]
extern crate lazy_static;

use gonk_player::Song;
use jwalk::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::*;
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

mod database;

pub mod playlist;
pub mod query;
pub use crate::database::*;

lazy_static! {
    pub static ref GONK_DIR: PathBuf = {
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
    pub static ref DB_DIR: PathBuf = GONK_DIR.join("gonk.db");
    pub static ref CONN: Mutex<Connection> = {
        let exists = PathBuf::from(DB_DIR.as_path()).exists();
        let conn = Connection::open(DB_DIR.as_path()).unwrap();

        if !exists {
            create_tables(&conn);
        };
        Mutex::new(conn)
    };
}

pub fn create_tables(conn: &Connection) {
    conn.execute(
        "CREATE TABLE settings (
             volume INTEGER UNIQUE,
             device TEXT UNIQUE)",
        [],
    )
    .unwrap();

    conn.execute("INSERT INTO settings (volume, device) VALUES (15, '')", [])
        .unwrap();

    conn.execute(
        "CREATE TABLE folder (
            folder TEXT PRIMARY KEY)",
        [],
    )
    .unwrap();

    conn.execute("CREATE TABLE persist(song_id INTEGER)", [])
        .unwrap();

    conn.execute(
        "CREATE TABLE song (
                name TEXT NOT NULL,
                disc INTEGER NOT NULL,
                number INTEGER NOT NULL,
                path TEXT NOT NULL,
                gain DOUBLE NOT NULL,
                album TEXT NOT NULL,
                artist TEXT NOT NULL,
                folder TEXT NOT NULL,
                FOREIGN KEY (folder) REFERENCES folder (folder),
                UNIQUE(name, disc, number, path, gain, album, artist, folder) ON CONFLICT REPLACE)",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE playlist (
            name TEXT PRIMARY KEY)",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE playlist_item (
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            album TEXT NOT NULL,
            artist TEXT NOT NULL,
            playlist_id TEXT NOT NULL,
            FOREIGN KEY (playlist_id) REFERENCES playlist (name))",
        [],
    )
    .unwrap();
}

pub fn reset() -> Result<(), &'static str> {
    *CONN.lock().unwrap() = Connection::open_in_memory().unwrap();

    if std::fs::remove_file(DB_DIR.as_path()).is_err() {
        Err("Could not remove database while it's in use.")
    } else {
        Ok(())
    }
}

pub fn conn() -> MutexGuard<'static, Connection> {
    CONN.lock().unwrap()
}

pub fn collect_songs(path: &str) -> Vec<Song> {
    let paths: Vec<_> = WalkDir::new(path)
        .into_iter()
        .flatten()
        .map(|dir| dir.path())
        .filter(|path| match path.extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
            }
            None => false,
        })
        .collect();

    paths
        .par_iter()
        .flat_map(|path| Song::from(&path))
        .collect()
}

pub fn create_batch_query(table: &str, folder: &str, songs: &[Song]) -> String {
    let queries: Vec<String> = songs
        .iter()
        .map(|song| {
            let name = song.name.replace('\'', r"''");
            let artist = song.artist.replace('\'', r"''");
            let album = song.album.replace('\'', r"''");
            let path = song.path.to_string_lossy().replace('\'', r"''");
            let folder = folder.replace('\'', r"''");

            format!(
                "INSERT OR IGNORE INTO {} (name, disc, number, path, gain, album, artist, folder)
                VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                table, name, song.disc, song.number, path, song.gain, album, artist, folder,
            )
        })
        .collect();

    format!("BEGIN;\n{}\nCOMMIT;", queries.join("\n"))
}

pub fn rescan_folders() {
    let folders = query::folders();
    let conn = conn();

    for folder in folders {
        let songs = collect_songs(&folder);
        let query = create_batch_query("song", &folder, &songs);
        conn.execute("DELETE FROM song", []).unwrap();
        conn.execute_batch(&query).unwrap();
    }
}

pub fn add_folder(folder: &str) {
    let folder = folder.replace("\\", "/");
    conn()
        .execute(
            "INSERT OR IGNORE INTO folder (folder) VALUES (?1)",
            [&folder],
        )
        .unwrap();

    let songs = collect_songs(&folder);
    let query = create_batch_query("song", &folder, &songs);

    conn().execute_batch(&query).unwrap();
}

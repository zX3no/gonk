#[macro_use]
extern crate lazy_static;

use gonk_player::Song;
use jwalk::WalkDir;
use rayon::iter::{ParallelBridge, ParallelIterator};
use rusqlite::*;
use std::{
    path::{Path, PathBuf},
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
                UNIQUE(name, disc, number, path, folder) ON CONFLICT REPLACE)",
                [],
            )
            .unwrap();

            conn.execute(
                "CREATE TABLE playlist (
            name TEXT PRIMARY KEY)",
                [],
            )
            .unwrap();

            //Used for intersects
            //https://www.sqlitetutorial.net/sqlite-intersect/
            conn.execute(
                "CREATE TABLE temp_song (
                name TEXT NOT NULL,
                disc INTEGER NOT NULL,
                number INTEGER NOT NULL,
                path TEXT NOT NULL,
                gain DOUBLE NOT NULL,
                album TEXT NOT NULL,
                artist TEXT NOT NULL,
                folder TEXT NOT NULL,
                FOREIGN KEY (folder) REFERENCES folder (folder),
                UNIQUE(name, disc, number, path, folder) ON CONFLICT REPLACE)",
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

        Mutex::new(conn)
    };
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

pub fn collect_songs(path: impl AsRef<Path>) -> Vec<Song> {
    WalkDir::new(path)
        .into_iter()
        .flatten()
        .map(|dir| dir.path())
        .filter(|path| match path.extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
            }
            None => false,
        })
        .par_bridge()
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
                "INSERT OR REPLACE INTO {} (name, disc, number, path, gain, album, artist, folder)
                VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                table, name, song.disc, song.number, path, song.gain, album, artist, folder,
            )
        })
        .collect();

    format!("BEGIN;\n{}\nCOMMIT;", queries.join("\n"))
}

pub fn rescan_folder(folder: &str) {
    let folder = folder.replace(r"\", r"/");
    //Make sure folder exists.
    if conn()
        .execute("INSERT INTO folder (folder) VALUES (?1)", [&folder])
        .is_err()
    {
        let songs = collect_songs(&folder);
        let temp_song_query = create_batch_query("temp_song", &folder, &songs);
        let song_query = create_batch_query("song", &folder, &songs);

        let conn = conn();
        conn.execute("DELETE FROM temp_song", []).unwrap();
        conn.execute_batch(&temp_song_query).unwrap();
        conn.execute_batch(&song_query).unwrap();

        //Drop the difference.
        conn.execute(
            "DELETE FROM song WHERE rowid IN (SELECT rowid FROM song EXCEPT SELECT rowid FROM temp_song)",
            [],
        ).unwrap();
    }
}

pub fn add_folder(folder: &str) {
    let folder = folder.replace(r"\", r"/");
    if conn()
        .execute("INSERT INTO folder (folder) VALUES (?1)", [&folder])
        .is_ok()
    {
        let songs = collect_songs(&folder);

        let query = create_batch_query("song", &folder, &songs);
        conn().execute_batch(&query).unwrap();
    } else {
        //TODO: Log to status bar that folder is already added
    }
}

use gonk_player::Song;
use lazy_static::lazy_static;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
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
    conn.execute_batch("PRAGMA synchronous = 0;").unwrap();

    conn.execute(
        "CREATE TABLE settings (
             volume  INTEGER UNIQUE,
             device  TEXT UNIQUE,
             selected   INTEGER UNIQUE,
             elapsed FLOAT UNIQUE)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO settings (volume, device, selected, elapsed) VALUES (15, '', 0, 0.0)",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE folder (
            folder TEXT PRIMARY KEY)",
        [],
    )
    .unwrap();

    conn.execute("CREATE TABLE queue(song_id INTEGER)", [])
        .unwrap();

    conn.execute(
        "CREATE TABLE song (
                name TEXT NOT NULL,
                disc INTEGER NOT NULL,
                number INTEGER NOT NULL,
                path TEXT NOT NULL,
                gain FLOAT NOT NULL,
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
        Err("Can't remove database while it's in use.")
    } else {
        Ok(())
    }
}

pub fn conn() -> MutexGuard<'static, Connection> {
    CONN.lock().unwrap()
}

pub fn collect_songs(path: &str) -> Vec<Song> {
    let paths: Vec<_> = walkdir::WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|path| match path.path().extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
            }
            None => false,
        })
        .collect();

    paths
        .into_par_iter()
        .flat_map(|path| Song::from(path.path()))
        .collect()
}

pub fn rescan_folders() {
    let folders = query::folders();

    for folder in folders {
        let songs = collect_songs(&folder);

        conn().execute("DELETE FROM song", []).unwrap();
        insert_songs(songs, &folder);
    }
}

pub fn add_folder(folder: &str) {
    let folder = folder.replace('/', "\\");

    conn()
        .execute(
            "INSERT OR IGNORE INTO folder (folder) VALUES (?1)",
            [&folder],
        )
        .unwrap();

    let songs = collect_songs(&folder);
    insert_songs(songs, &folder);
}

fn insert_songs(songs: Vec<Song>, folder: &str) {
    let mut conn = conn();
    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO song (name, disc, number, path, gain, album, artist, folder)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .unwrap();

        for song in songs {
            stmt.execute(params![
                &song.name,
                &song.disc,
                &song.number,
                &song.path.to_string_lossy(),
                &song.gain,
                &song.album,
                &song.artist,
                &folder,
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

#[macro_use]
extern crate lazy_static;

use gonk_player::Song;
use jwalk::WalkDir;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    slice::ParallelSliceMut,
};
use rusqlite::*;
use std::{
    path::{Path, PathBuf},
    sync::{RwLock, RwLockReadGuard},
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
}

pub fn init() {
    let exists = PathBuf::from(DB_DIR.as_path()).exists();
    let conn = Connection::open(DB_DIR.as_path()).unwrap();

    if !exists {
        conn.execute(
            "CREATE TABLE settings (
             volume INTEGER UNIQUE,
             device TEXT UNIQUE
        )",
            [],
        )
        .unwrap();

        conn.execute("INSERT INTO settings (volume, device) VALUES (15, '')", [])
            .unwrap();

        conn.execute(
            "CREATE TABLE folder (
            path TEXT PRIMARY KEY
        )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE artist (
            name TEXT PRIMARY KEY
        )",
            [],
        )
        .unwrap();

        conn.execute("CREATE TABLE persist(song_id INTEGER)", [])
            .unwrap();

        conn.execute(
            "CREATE TABLE album (
            name TEXT PRIMARY KEY,
            artist_id TEXT NOT NULL,
            FOREIGN KEY (artist_id) REFERENCES artist (name) 
        )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE song (
            name TEXT NOT NULL,
            disc INTEGER NOT NULL,
            number INTEGER NOT NULL,
            path TEXT NOT NULL,
            gain DOUBLE NOT NULL,
            album_id TEXT NOT NULL,
            artist_id TEXT NOT NULL,
            folder_id TEXT NOT NULL,
            FOREIGN KEY (album_id) REFERENCES album (name),
            FOREIGN KEY (artist_id) REFERENCES artist (name),
            FOREIGN KEY (folder_id) REFERENCES folder (path),
            UNIQUE(name, disc, number, path, album_id, artist_id, folder_id) ON CONFLICT REPLACE
        )",
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
            album_id TEXT NOT NULL,
            artist_id TEXT NOT NULL,
            folder_id TEXT NOT NULL,
            FOREIGN KEY (album_id) REFERENCES album (name),
            FOREIGN KEY (artist_id) REFERENCES artist (name),
            FOREIGN KEY (folder_id) REFERENCES folder (path) 
        )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE playlist (
            name TEXT PRIMARY KEY
        )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE playlist_item (
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            album_id TEXT NOT NULL,
            artist_id TEXT NOT NULL,
            playlist_id TEXT NOT NULL,
            FOREIGN KEY (album_id) REFERENCES album (name),
            FOREIGN KEY (artist_id) REFERENCES artist (name),
            FOREIGN KEY (playlist_id) REFERENCES playlist (name)
        )",
            [],
        )
        .unwrap();
    }

    unsafe {
        CONN = Some(RwLock::new(conn));
    }
}

pub static mut CONN: Option<RwLock<rusqlite::Connection>> = None;

pub fn reset() -> Result<(), &'static str> {
    unsafe {
        CONN = None;
    }

    if std::fs::remove_file(DB_DIR.as_path()).is_err() {
        Err("Could not remove database while it's in use.")
    } else {
        Ok(())
    }
}

pub fn conn() -> RwLockReadGuard<'static, Connection> {
    unsafe { CONN.as_ref().unwrap().read().unwrap() }
}

pub fn collect_songs(path: impl AsRef<Path>) -> Vec<Song> {
    //TODO: Check if the path is in the database and if it is, don't read the metadata.
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

pub fn insert_parents(songs: &[Song]) {
    let mut albums: Vec<(&str, &str)> = songs
        .par_iter()
        .map(|song| (song.album.as_str(), song.artist.as_str()))
        .collect();

    albums.par_sort();
    albums.dedup();

    let mut artists: Vec<&str> = songs.par_iter().map(|song| song.artist.as_str()).collect();

    artists.par_sort();
    artists.dedup();

    let query: String = artists
        .par_iter()
        .map(|artist| {
            let artist = artist.replace('\'', r"''");
            format!("INSERT OR IGNORE INTO artist (name) VALUES ('{}');", artist)
        })
        .collect::<Vec<String>>()
        .join("\n");

    let query = format!("BEGIN;\n{}\nCOMMIT;", query);
    conn().execute_batch(&query).unwrap();

    let query: Vec<String> = albums
        .par_iter()
        .map(|(album, artist)| {
            let artist = artist.replace('\'', r"''");
            let album = album.replace('\'', r"''");
            format!(
                "INSERT OR IGNORE INTO album (name, artist_id) VALUES ('{}', '{}');",
                album, artist
            )
        })
        .collect();

    let query = format!("BEGIN;\n{}\nCOMMIT;", query.join("\n"));
    conn().execute_batch(&query).unwrap();
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
                "INSERT OR REPLACE INTO {} (name, disc, number, path, gain, album_id, artist_id, folder_id)
                VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                table, name, song.disc, song.number, path, song.gain, album, artist, folder,
            )
        })
        .collect();

    format!("BEGIN;\n{}\nCOMMIT;", queries.join("\n"))
}

pub fn rescan_folder(folder: &str) {
    //Make sure folder exists.
    if conn()
        .execute("INSERT INTO folder (path) VALUES (?1)", [folder])
        .is_err()
    {
        //Collect the songs.
        let songs = collect_songs(folder);
        insert_parents(&songs);

        //Create query.
        let query = create_batch_query("temp_song", folder, &songs);

        let conn = conn();

        //Clean the temp table and add songs.
        conn.execute("DELETE FROM temp_song", []).unwrap();
        conn.execute_batch(&query).unwrap();

        //Insert songs into default table.
        let query = create_batch_query("song", folder, &songs);
        conn.execute_batch(&query).unwrap();

        //Drop the difference.
        conn.execute(
            "DELETE FROM song WHERE rowid IN (SELECT rowid FROM song EXCEPT SELECT rowid FROM temp_song)",
            [],
        ).unwrap();
    }
}

pub fn add_folder(folder: &str) {
    if conn()
        .execute("INSERT INTO folder (path) VALUES (?1)", [folder])
        .is_ok()
    {
        let songs = collect_songs(folder);
        insert_parents(&songs);

        let query = create_batch_query("song", folder, &songs);
        conn().execute_batch(&query).unwrap();
    }
}

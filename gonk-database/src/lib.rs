#![allow(unused)]
use gonk_player::Song;
use jwalk::WalkDir;
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    slice::ParallelSliceMut,
};
use rusqlite::*;
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard},
};

pub mod query;

#[derive(Debug)]
pub struct Artist {
    name: String,
}

#[derive(Debug)]
pub struct Album {
    name: String,
    artist: String,
}

#[derive(Debug)]
pub struct Folder {
    pub path: String,
}

#[must_use]
pub fn init() -> Result<()> {
    let exists = PathBuf::from("gonk.db").exists();
    let conn = Connection::open("gonk.db")?;

    if !exists {
        conn.execute(
            "CREATE TABLE folder (
            path TEXT PRIMARY KEY
        )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE artist (
            name TEXT PRIMARY KEY
        )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE album (
            name TEXT PRIMARY KEY,
            artist_id TEXT NOT NULL,
            FOREIGN KEY (artist_id) REFERENCES artist (name) 
        )",
            [],
        )?;

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
            FOREIGN KEY (folder_id) REFERENCES folder (path) 
        )",
            [],
        )?;

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
        )?;
    }

    unsafe {
        CONN = Some(RwLock::new(conn));
    }

    Ok(())
}

pub static mut CONN: Option<RwLock<rusqlite::Connection>> = None;

pub fn reset() {
    unsafe {
        CONN = None;
    }
    let _ = std::fs::remove_file("gonk.db");
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
            format!("INSERT INTO artist (name) VALUES ('{}');", artist)
        })
        .collect::<Vec<String>>()
        .join("\n");

    let query = format!("BEGIN;\n{}\nCOMMIT;", query);
    conn().execute_batch(&query).unwrap();

    let query: Vec<String> = albums
        .par_iter()
        .map(|((album, artist))| {
            let artist = artist.replace('\'', r"''");
            let album = album.replace('\'', r"''");
            format!(
                "INSERT INTO album (name, artist_id) VALUES ('{}', '{}');",
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
                "INSERT INTO {} (name, disc, number, path, gain, album_id, artist_id, folder_id)
                VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                table, name, song.disc, song.number, path, song.gain, album, artist, folder,
            )
        })
        .collect();

    format!("BEGIN;\n{}\nCOMMIT;", queries.join("\n"))
}

#[must_use]
pub fn rescan_folder(folder: &str) -> Result<()> {
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
        conn.execute("DELETE FROM temp_song", [])?;
        conn.execute_batch(&query);

        //Insert songs into default table.
        let query = create_batch_query("song", folder, &songs);
        conn.execute_batch(&query);

        //Drop the difference.
        let result = conn.execute(
            "DELETE FROM song WHERE rowid IN (SELECT rowid FROM song EXCEPT SELECT rowid FROM temp_song)",
            [],
        )?;
    }

    Ok(())
}

pub fn add_folder(folder: &str) {
    conn()
        .execute("INSERT INTO folder (path) VALUES (?1)", [folder])
        .unwrap();

    let songs = collect_songs(folder);
    insert_parents(&songs);

    let query = create_batch_query("song", folder, &songs);
    conn().execute_batch(&query).unwrap();
}

use crate::{Song, DB_DIR};
use jwalk::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection, Params, Row};
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    thread::{self, JoinHandle},
    time::Duration,
};

pub fn get_all_songs() -> Vec<(usize, Song)> {
    let conn = conn();
    let mut stmt = conn.prepare("SELECT *, rowid FROM song").unwrap();

    stmt.query_map([], |row| {
        let id = row.get(9).unwrap();
        let song = song(row);
        Ok((id, song))
    })
    .unwrap()
    .flatten()
    .collect()
}
pub fn get_all_artists() -> Vec<String> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")
        .unwrap();

    stmt.query_map([], |row| {
        let artist: String = row.get(0).unwrap();
        Ok(artist)
    })
    .unwrap()
    .flatten()
    .collect()
}
pub fn get_all_albums() -> Vec<(String, String)> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT album, artist FROM song ORDER BY artist COLLATE NOCASE")
        .unwrap();

    stmt.query_map([], |row| {
        let album: String = row.get(0).unwrap();
        let artist: String = row.get(1).unwrap();
        Ok((album, artist))
    })
    .unwrap()
    .flatten()
    .collect()
}
pub fn get_all_albums_by_artist(artist: &str) -> Vec<String> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT album FROM song WHERE artist = ? ORDER BY album COLLATE NOCASE")
        .unwrap();

    stmt.query_map([artist], |row| row.get(0))
        .unwrap()
        .flatten()
        .collect()
}
pub fn get_all_songs_from_album(album: &str, artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT * FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number",
        params![artist, album],
    )
}
pub fn get_songs_by_artist(artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT * FROM song WHERE artist = ? ORDER BY album, disc, number",
        params![artist],
    )
}
pub fn get_song(song: &(u64, String), album: &str, artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT * FROM song WHERE name=(?1) AND number=(?2) AND artist=(?3) AND album=(?4)",
        params![song.1, song.0, artist, album],
    )
}
pub fn get_songs_from_id(ids: &[usize]) -> Vec<Song> {
    ids.iter()
        .filter_map(|id| {
            collect_songs("SELECT * FROM song WHERE rowid = ?", params![id])
                .first()
                .cloned()
        })
        .collect()
}
fn collect_songs<P>(query: &str, params: P) -> Vec<Song>
where
    P: Params,
{
    let conn = conn();
    let mut stmt = conn.prepare(query).unwrap();

    stmt.query_map(params, |row| Ok(song(row)))
        .unwrap()
        .flatten()
        .collect()
}
fn song(row: &Row) -> Song {
    let path: String = row.get(5).unwrap();
    let dur: f64 = row.get(6).unwrap();
    Song {
        number: row.get(0).unwrap(),
        disc: row.get(1).unwrap(),
        name: row.get(2).unwrap(),
        album: row.get(3).unwrap(),
        artist: row.get(4).unwrap(),
        duration: Duration::from_secs_f64(dur),
        path: PathBuf::from(path),
        track_gain: row.get(7).unwrap(),
    }
}

pub static mut CONN: Option<Mutex<rusqlite::Connection>> = None;

pub fn conn() -> MutexGuard<'static, Connection> {
    unsafe { CONN.as_ref().unwrap().lock().unwrap() }
}

#[allow(unused)]
pub fn reset() {
    unsafe {
        CONN = None;
    }
    std::fs::remove_file(DB_DIR.as_path());
}

pub fn add_playlist(name: &str, songs: &[usize]) {
    let conn = conn();
    for song in songs {
        conn.execute(
            "INSERT OR IGNORE INTO playlist VALUES (?1, ?2)",
            params![song, name],
        )
        .unwrap();
    }
}

pub mod playlist {
    use super::conn;

    pub fn get_names() -> Vec<String> {
        let conn = conn();
        let mut stmt = conn.prepare("SELECT DISTINCT name FROM playlist").unwrap();

        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect()
    }

    pub fn get(playlist_name: &str) -> (Vec<usize>, Vec<usize>) {
        //TODO: playlist should reference song
        let conn = conn();
        let mut stmt = conn
            .prepare("SELECT rowid, song_id FROM playlist WHERE name = ?")
            .unwrap();

        let ids: Vec<_> = stmt
            .query_map([playlist_name], |row| {
                let row_id: usize = row.get(0).unwrap();
                let song_id: usize = row.get(1).unwrap();
                Ok((row_id, song_id))
            })
            .unwrap()
            .flatten()
            .collect();

        let row_ids: Vec<_> = ids.iter().map(|id| id.0).collect();
        let song_ids: Vec<_> = ids.iter().map(|id| id.1).collect();
        (row_ids, song_ids)
    }

    pub fn remove(id: usize) {
        conn()
            .execute("DELETE FROM playlist WHERE rowid = ?", [id])
            .unwrap();
    }
}

pub fn open_database() -> Option<Mutex<rusqlite::Connection>> {
    let exists = DB_DIR.exists();
    if let Ok(conn) = Connection::open(DB_DIR.as_path()) {
        if !exists {
            conn.execute(
                "CREATE TABLE song (
                    number     INTEGER NOT NULL,
                    disc       INTEGER NOT NULL,
                    name       TEXT NOT NULL,
                    album      TEXT NOT NULL,
                    artist     TEXT NOT NULL,
                    path       TEXT NOT NULL UNIQUE,
                    duration   DOUBLE NOT NULL,
                    track_gain DOUBLE NOT NULL,
                    parent     TEXT NOT NULL
                )",
                [],
            )
            .unwrap();

            conn.execute(
                "CREATE TABLE playlist (
                    song_id INTEGER NOT NULL,
                    name TEXT NOT NULL
                )",
                [],
            )
            .unwrap();
        }
        Some(Mutex::new(conn))
    } else {
        None
    }
}

pub enum State {
    Busy,
    Idle,
    NeedsUpdate,
}

#[derive(Default)]
pub struct Database {
    handle: Option<JoinHandle<()>>,
}

impl Database {
    pub fn add_paths(&mut self, paths: &[String]) {
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                return;
            }
        }

        let paths = paths.to_vec();

        let handle = thread::spawn(move || {
            let queries: Vec<String> = paths
                .iter()
                .map(|path| {
                    let paths: Vec<PathBuf> = WalkDir::new(path)
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

                    let songs: Vec<Song> = paths
                        .par_iter()
                        .map(|dir| Song::from(dir))
                        .flatten()
                        .collect();

                    if songs.is_empty() {
                        String::new()
                    } else {
                        songs
                            .iter()
                            .map(|song| {
                                let artist = song.artist.replace('\'', r"''");
                                let album = song.album.replace('\'', r"''");
                                let name = song.name.replace('\'', r"''");
                                let song_path= song.path.to_string_lossy().replace('\'', r"''");
                                let parent = path.replace('\'', r"''");

                                format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, duration, track_gain, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                            song.number, song.disc, name, album, artist, song_path, song.duration.as_secs_f64(), song.track_gain, parent)
                            })
                            .collect::<Vec<String>>()
                            .join("\n")
                    }
                })
                .collect();

            let stmt = format!("BEGIN;\nDELETE FROM song;\n{}COMMIT;\n", queries.join("\n"));
            conn().execute_batch(&stmt).unwrap();
        });

        self.handle = Some(handle);
    }
    pub fn state(&mut self) -> State {
        match self.handle {
            Some(ref handle) => {
                let finished = handle.is_finished();
                if finished {
                    self.handle = None;
                    State::NeedsUpdate
                } else {
                    State::Busy
                }
            }
            None => State::Idle,
        }
    }
}

use gronk_types::Song;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use walkdir::WalkDir;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref CONFIG_DIR: PathBuf = {
        let config_dir = dirs::config_dir().unwrap();
        config_dir.join("gronk")
    };
    static ref DB_DIR: PathBuf = {
        let db_dir = dirs::config_dir().unwrap();
        db_dir.join("gronk\\gronk.db")
    };
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> rusqlite::Result<Self> {
        if !Path::new(CONFIG_DIR.as_path()).exists() {
            std::fs::create_dir(CONFIG_DIR.as_path()).unwrap();
        }

        if !Path::new(DB_DIR.as_path()).exists() {
            let conn = Connection::open(DB_DIR.as_path()).unwrap();
            conn.busy_timeout(Duration::from_millis(0))?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "synchronous", "0")?;
            conn.pragma_update(None, "temp_store", "MEMORY")?;

            conn.execute(
                "CREATE VIRTUAL TABLE song USING FTS4(
                    number  INTEGER NOT NULL,
                    disc    INTEGER NOT NULL,
                    name    TEXT NOT NULL,
                    album   TEXT NOT NULL,
                    artist  TEXT NOT NULL,
                    path    TEXT NOT NULL
                )",
                [],
            )?;

            conn.execute(
                "CREATE TABLE music(
                    path TEXT NOT NULL,
                    UNIQUE(path)
                )",
                [],
            )
            .unwrap();

            conn.execute(
                "CREATE TABLE config(
                    volume INTEGER NOT NULL UNIQUE
                )",
                [],
            )
            .unwrap();

            conn.execute("INSERT OR IGNORE INTO config (volume) VALUES (15)", [])
                .unwrap();
        }

        Ok(Self {
            conn: Connection::open(DB_DIR.as_path()).unwrap(),
        })
    }
    pub fn get_volume(&self) -> u16 {
        let mut stmt = self.conn.prepare("SELECT volume FROM config").unwrap();
        let mut rows = stmt.query([]).unwrap();
        if let Some(row) = rows.next().unwrap() {
            row.get(0).unwrap()
        } else {
            panic!();
        }
    }
    pub fn set_volume(&self, positive: bool) {
        let mut volume = self.get_volume();
        if positive {
            if volume != 100 {
                volume += 5;
            }
        } else {
            if volume != 0 {
                volume -= 5;
            }
        }

        self.conn
            .execute("UPDATE config SET volume = (?1)", [volume])
            .unwrap();
    }
    pub fn add_music(&self, music_dir: &str) {
        let paths: Vec<PathBuf> = WalkDir::new(music_dir)
            .into_iter()
            .filter_map(|entry| {
                if let Some(ex) = entry.as_ref().unwrap().path().extension() {
                    if ex == "flac" || ex == "mp3" || ex == "m4a" {
                        return Some(entry.as_ref().unwrap().path().to_path_buf());
                    }
                }
                None
            })
            .collect();

        let songs: Vec<Song> = paths
            .par_iter()
            .map(|path| Song::from(path.to_str().unwrap()))
            .collect();

        let sqlite_connection_manager = SqliteConnectionManager::file(DB_DIR.as_path());
        let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager).unwrap();

        let pool_arc = Arc::new(sqlite_pool);

        songs.par_iter().for_each(|song| {
                    let connection = pool_arc.get().unwrap();
                    connection
                        .execute(
                            "INSERT INTO song (number, disc, name, album, artist, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![song.number, song.disc, song.name, song.album, song.artist, song.path.to_str().unwrap()],
                        ).unwrap();
                });

        self.conn
            .execute("INSERT INTO song(song) VALUES('optimize')", [])
            .unwrap();
    }
    pub fn add_dir(&self, music_dir: &str) {
        let conn = Connection::open(DB_DIR.as_path()).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO music (path) VALUES (?1)",
            params![music_dir],
        )
        .unwrap();
        self.add_music(music_dir);
    }
    pub fn is_empty(&self) -> bool {
        let mut stmt = self.conn.prepare("SELECT * FROM music").unwrap();
        let mut rows = stmt.query([]).unwrap();
        if let Some(_) = rows.next().unwrap() {
            false
        } else {
            true
        }
    }
    pub fn reset(&self) {
        todo!();
        // let mut stmt = self.conn.prepare("SELECT * FROM config").unwrap();
        // let dirs: Vec<String> = stmt
        //     .query_map([], |row| {
        //         let dir: String = row.get(0).unwrap();
        //         Ok(dir)
        //     })
        //     .unwrap()
        //     .flatten()
        //     .collect();
        // self.conn.execute("DROP TABLE song", []).unwrap();
        // for dir in dirs {
        //     self.add_music(&dir);
        // }
    }
    pub fn get_songs_from_ids(&self, ids: &[usize]) -> Vec<Song> {
        if ids.is_empty() {
            return Vec::new();
        }

        let mut songs = Vec::new();

        for id in ids {
            let query = format!("SELECT * FROM song WHERE rowid='{}'", id);
            let mut stmt = self.conn.prepare(&query).unwrap();
            let mut rows = stmt.query([]).unwrap();
            if let Some(row) = rows.next().unwrap() {
                let path: String = row.get(5).unwrap();
                songs.push(Song {
                    number: row.get(0).unwrap(),
                    disc: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    album: row.get(3).unwrap(),
                    artist: row.get(4).unwrap(),
                    path: PathBuf::from(path),
                });
            }
        }
        songs
    }
    pub fn get_song_from_id(&self, id: usize) -> Song {
        let query = format!("SELECT * FROM song WHERE rowid='{}'", id);
        let mut stmt = self.conn.prepare(&query).unwrap();
        let mut rows = stmt.query([]).unwrap();

        if let Some(row) = rows.next().unwrap() {
            let path: String = row.get(5).unwrap();
            Song {
                number: row.get(0).unwrap(),
                disc: row.get(1).unwrap(),
                name: row.get(2).unwrap(),
                album: row.get(3).unwrap(),
                artist: row.get(4).unwrap(),
                path: PathBuf::from(path),
            }
        } else {
            panic!();
        }
    }
    pub fn get_songs(&self) -> Vec<(Song, usize)> {
        let mut stmt = self.conn.prepare("SELECT rowid, * FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(0).unwrap();
            let path: String = row.get(6).unwrap();
            Ok((
                Song {
                    number: row.get(1).unwrap(),
                    disc: row.get(2).unwrap(),
                    name: row.get(3).unwrap(),
                    album: row.get(4).unwrap(),
                    artist: row.get(5).unwrap(),
                    path: PathBuf::from(path),
                },
                id,
            ))
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn search(&self, search: String) -> Vec<Song> {
        let query = if !search.is_empty() {
            let mut search = search.replace("\'", "\'\'");
            while search.ends_with(' ') {
                search.pop();
            }
            format!(
                "SELECT * FROM song WHERE song MATCH '\"{}*\"' ORDER BY artist, album, disc, number COLLATE NOCASE",
              search,
            )
        } else {
            "SELECT * FROM song ORDER BY artist, album, disc, number COLLATE NOCASE".to_string()
        };
        self.collect_songs(&query)
    }
    pub fn artists(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")?;

        let mut rows = stmt.query([])?;

        let mut artists = Vec::new();
        while let Some(row) = rows.next()? {
            let artist: String = row.get(0)?;
            artists.push(artist);
        }
        Ok(artists)
    }
    pub fn albums(&self) -> Vec<(String, String)> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT album, artist FROM song ORDER BY artist COLLATE NOCASE")
            .unwrap();

        let mut rows = stmt.query([]).unwrap();

        let mut albums = Vec::new();

        while let Some(row) = rows.next().unwrap() {
            let album: String = row.get(0).unwrap();
            let artist: String = row.get(1).unwrap();
            albums.push((album, artist));
        }
        albums
    }
    pub fn albums_by_artist(&self, artist: &str) -> Result<Vec<String>> {
        let artist = fix(artist);

        let query = format!(
            "SELECT DISTINCT album FROM song WHERE artist = '{}' ORDER BY album COLLATE NOCASE",
            artist
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        let mut albums = Vec::new();
        while let Some(row) = rows.next()? {
            let album: String = row.get(0)?;
            albums.push(album);
        }

        Ok(albums)
    }
    pub fn songs_from_album(&self, artist: &str, album: &str) -> Result<Vec<(u16, String)>> {
        let artist = fix(artist);
        let album = fix(album);

        let query = format!(
            "SELECT number, name FROM song WHERE song MATCH 'artist:{} AND album:{}' ORDER BY disc, number",
            artist, album
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        let mut songs = Vec::new();
        while let Some(row) = rows.next()? {
            let number: u16 = row.get(0)?;
            let name: String = row.get(1)?;
            songs.push((number, name));
        }

        Ok(songs)
    }
    pub fn get_artist(&self, artist: &str) -> Vec<Song> {
        let artist = fix(artist);

        let query = format!(
            "SELECT * FROM song WHERE artist = '{}' ORDER BY album, disc, number",
            artist,
        );

        self.collect_songs(&query)
    }
    pub fn get_album(&self, album: &str, artist: &str) -> Vec<Song> {
        let album = fix(album);
        let artist = fix(artist);

        //the order of the query is important? artist must come first
        let query = format!(
            "SELECT * FROM song WHERE song MATCH 'artist:{} album:{}' ORDER BY disc, number",
            artist, album
        );

        self.collect_songs(&query)
    }
    pub fn get_song(&self, artist: &str, album: &str, number: &u16, name: &str) -> Vec<Song> {
        let artist = fix(artist);
        let album = fix(album);
        let name = fix(name);

        //TODO: benchmark queries and swap to fts4 for others
        //TODO: Disc number too?
        let query = format!(
            "SELECT * FROM song WHERE song MATCH 'name:{} AND number:{} AND artist:{} AND album:{}'",
            name, number, artist, album
        );

        self.collect_songs(&query)
    }

    fn collect_songs(&self, query: &str) -> Vec<Song> {
        let mut stmt = self.conn.prepare(query).unwrap();

        stmt.query_map([], |row| {
            let path: String = row.get(5).unwrap();
            Ok(Song {
                number: row.get(0).unwrap(),
                disc: row.get(1).unwrap(),
                name: row.get(2).unwrap(),
                album: row.get(3).unwrap(),
                artist: row.get(4).unwrap(),
                path: PathBuf::from(path),
            })
        })
        .unwrap()
        .flatten()
        .collect()
    }
}
fn fix(item: &str) -> String {
    item.replace("\'", "\'\'").replace(":", "\\:")
}

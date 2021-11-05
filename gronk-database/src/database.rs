use audiotags::Tag;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection, Result};
use std::{fs::File, path::PathBuf, sync::Arc};
use walkdir::WalkDir;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Self {
        Self {
            conn: Connection::open("music.db").unwrap(),
        }
    }

    pub fn create_db(&self) -> rusqlite::Result<()> {
        let conn = &self.conn;

        File::create("music.db").unwrap();
        conn.execute("PRAGMA foregin_keys = ON", [])?;
        // conn.execute("PRAGMA journal_mode = MEMORY", [])?;
        conn.execute("PRAGMA synchronous = 0", [])?;
        // conn.execute("PRAGMA cache_size = 1000000", [])?;
        conn.execute("PRAGMA temp_store = MEMORY", [])?;
        // conn.execute("PRAGMA busy_timeout = 5000", [])?;

        conn.execute(
            "CREATE TABLE song(
                    name    TEXT,
                    path    TEXT,
                    track   INTEGER,
                    disc    INTEGER,
                    album   TEXT,
                    artist  TEXT
                )",
            [],
        )?;

        let paths: Vec<PathBuf> = WalkDir::new("D:/OneDrive/Music")
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

        let songs: Vec<MinSong> = paths
            .par_iter()
            .map(|path| MinSong::from(path.to_str().unwrap()))
            .collect();

        let sqlite_connection_manager = SqliteConnectionManager::file("music.db");
        let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager).unwrap();
        let pool_arc = Arc::new(sqlite_pool);

        let pool = pool_arc.clone();

        songs.par_iter().for_each(|song| {
            let connection = pool.get().unwrap();

            connection
                .execute(
                    "INSERT INTO song (track, disc, name, album, artist) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![song.track, song.disc, song.name, song.album, song.artist],
                )
                .unwrap();
        });

        Ok(())
    }
    pub fn first_artist(&self) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")?;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let artist: String = row.get(0)?;
            return Ok(artist);
        }
        panic!("no artists");
    }
    pub fn first_album(&self, artist: &str) -> Result<String> {
        let query = format!(
            "SELECT DISTINCT album FROM song WHERE artist = '{}' ORDER BY album COLLATE NOCASE",
            artist
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let album: String = row.get(0)?;
            return Ok(album);
        }
        panic!("no albums");
    }
    pub fn first_song(&self, artist: &str, album: &str) -> Result<String> {
        let query = format!(
            "SELECT track, name FROM song WHERE artist = '{}' AND album = '{}' ORDER BY disc, track",
            artist, album
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let track: usize = row.get(0)?;
            let name: String = row.get(1)?;
            return Ok(format!("{}: {}", track, name));
        }

        panic!("no albums");
    }

    pub fn get_artists(&self) -> Result<Vec<String>> {
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

    pub fn get_albums_by_artist(&self, artist: &str) -> Result<Vec<String>> {
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

    pub fn get_songs_from_album(&self, artist: &str, album: &str) -> Result<Vec<String>> {
        let query = format!(
            "SELECT track, name FROM song WHERE artist = '{}' AND album = '{}' ORDER BY disc, track",
            artist, album
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        let mut songs = Vec::new();
        while let Some(row) = rows.next()? {
            let track: usize = row.get(0)?;
            let name: String = row.get(1)?;
            songs.push(format!("{}. {}", track, name));
        }

        Ok(songs)
    }
}

#[derive(Debug, Clone)]
pub struct MinSong {
    path: PathBuf,
    album: String,
    artist: String,
    name: String,
    disc: u16,
    track: u16,
}

impl MinSong {
    pub fn from(path: &str) -> Self {
        //this is slow
        if let Ok(tag) = Tag::new().read_from_path(&path) {
            let artist = if let Some(artist) = tag.album_artist() {
                artist.to_string()
            } else if let Some(artist) = tag.artist() {
                artist.to_string()
            } else {
                panic!("no artist for {:?}", path);
            };
            let disc = tag.disc_number().unwrap_or(1);
            return MinSong {
                album: tag.album_title().unwrap().to_string(),
                artist,
                path: PathBuf::from(path),
                disc,
                name: tag.title().unwrap().to_string(),
                track: tag.track_number().unwrap(),
            };
        }
        panic!();
    }
}

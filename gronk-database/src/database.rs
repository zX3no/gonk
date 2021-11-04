use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard},
    thread::{self, JoinHandle},
    time::Instant,
};

use audiotags::Tag;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection, Result};
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
                    album   TEXT,
                    artist  TEXT
                )",
            [],
        )?;

        Ok(())
    }
    pub fn scan(path: &str) -> Vec<PathBuf> {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| {
                if let Some(ex) = entry.as_ref().unwrap().path().extension() {
                    if ex == "flac" || ex == "mp3" || ex == "m4a" {
                        return Some(entry.as_ref().unwrap().path().to_path_buf());
                    }
                }
                None
            })
            .collect()
    }
    pub fn write() {
        let p = "D:/OneDrive/Music/";
        let paths = Database::scan(p);

        let songs: Vec<MinSong> = paths
            .par_iter()
            .map(|path| MinSong::from(path.to_str().unwrap()))
            .collect();

        let sqlite_connection_manager = SqliteConnectionManager::file("music.db");
        let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager)
            .expect("Failed to create r2d2 SQLite connection pool");
        let pool_arc = Arc::new(sqlite_pool);

        let pool = pool_arc.clone();

        songs.par_iter().for_each(|song| {
            let connection = pool.get().unwrap();

            connection
                .execute(
                    "INSERT INTO song (track, name, album, artist) VALUES (?1, ?2, ?3, ?4)",
                    params![song.track, song.name, song.album, song.artist],
                )
                .unwrap();
        });
    }

    pub fn get_songs(&self) -> rusqlite::Result<()> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, album, track, artist FROM song WHERE artist = 'JPEGMAFIA'")
            .unwrap();

        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let album: String = row.get(1)?;
            let number: usize = row.get(2)?;
            let artist: String = row.get(3)?;

            println!("{} | {} | {} | {} ", number, name, album, artist);
        }
        Ok(())
    }
    pub fn get_artists(&self) -> Result<()> {
        let mut stmt = self.conn.prepare("SELECT artist FROM song")?;

        let mut rows = stmt.query([])?;

        let mut vec = Vec::new();
        while let Some(row) = rows.next()? {
            let s: String = row.get(0)?;
            vec.push(s);
        }
        vec.sort_by_key(|s| s.to_lowercase());
        vec.dedup();
        dbg!(vec);
        Ok(())
    }
    pub fn get_album_by_artist(&self, artist: &str) -> Result<()> {
        let query = format!("SELECT album FROM song WHERE artist = '{}'", artist);
        let mut stmt = self.conn.prepare(&query)?;

        let mut rows = stmt.query([])?;

        let mut vec = Vec::new();
        while let Some(row) = rows.next()? {
            let s: String = row.get(0)?;
            vec.push(s);
        }
        vec.sort_by_key(|s| s.to_lowercase());
        vec.dedup();
        dbg!(vec);
        Ok(())
    }
    pub fn get_songs_from_album(&self, album: &str, artist: &str) -> Result<()> {
        let query = format!(
            "SELECT name, track FROM song WHERE artist = '{}' AND album = '{}'",
            artist, album
        );
        let mut stmt = self.conn.prepare(&query)?;

        let mut rows = stmt.query([])?;

        let mut vec = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let track: usize = row.get(1)?;
            vec.push((name, track));
        }
        for song in vec {
            println!("{}: {}", song.1, song.0);
        }
        Ok(())
    }

    pub fn add_song(
        conn: Connection,
        track: u16,
        name: &str,
        album: &str,
        artist: &str,
        path: &str,
    ) -> rusqlite::Result<()> {
        let now = Instant::now();
        //oh yeah this is what i'm talking about
        //2 seconds for all songs
        //in a thread pool it should be fast
        //i think sqlite was build for that
        let query = format!(
            "BEGIN; 
            INSERT OR IGNORE INTO artist(id) VALUES ('{}'); 
            INSERT OR IGNORE INTO album(name, artist) VALUES ('{}', '{}'); 
            INSERT INTO song (track, name, path, album) VALUES('{}','{}','{}', (SELECT id FROM album WHERE name = '{}' AND artist = '{}'));
            COMMIT;",
            artist, album, artist,
            track, name, path, album, artist,
        );
        conn.execute_batch(&query)?;
        println!("{:?}", now.elapsed());

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MinSong {
    path: PathBuf,
    album: String,
    artist: String,
    name: String,
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
            return MinSong {
                album: tag.album_title().unwrap().to_string(),
                artist,
                path: PathBuf::from(path),
                name: tag.title().unwrap().to_string(),
                track: tag.track_number().unwrap(),
            };
        }
        panic!();
    }
}

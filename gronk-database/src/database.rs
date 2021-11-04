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
use rusqlite::Connection;
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

        conn.execute(
            "CREATE TABLE artist(
                    id TEXT PRIMARY KEY
                )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE album(
                    id INTEGER PRIMARY KEY,
                    name TEXT,
                    artist      INTEGER NOT NULL,
                    FOREIGN     KEY(artist) REFERENCES artist(id)
                )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE song(
                    name    TEXT,
                    path    TEXT,
                    track   INTEGER,
                    album   INTEGER NOT NULL REFERENCES album(id)
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
        let p = "D:/OneDrive/Music";
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
            let connection = pool.get();

            //this won't work because it needs to be executed in order
            let query = format!(
            "BEGIN; 
            INSERT OR IGNORE INTO artist(id) VALUES ('{}'); 
            INSERT OR IGNORE INTO album(name, artist) VALUES ('{}', '{}'); 
            INSERT INTO song (track, name, path, album) VALUES('{}','{}','{}', (SELECT id FROM album WHERE name = '{}' AND artist = '{}'));
            COMMIT;",
            song.artist, song.album, song.artist,
            song.track, song.name, song.path.to_str().unwrap(), song.album, song.artist,
        );
            connection.unwrap().execute_batch(&query).unwrap();
        });
    }

    pub fn get_all_songs(&self) -> rusqlite::Result<()> {
        let mut stmt = self.conn
        .prepare("SELECT song.name, song.album, path, track, artist.id FROM song INNER JOIN album ON song.album = album.id INNER JOIN artist ON album.artist = artist.id")
        .unwrap();

        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let album: usize = row.get(1)?;
            let path: String = row.get(2)?;
            let number: usize = row.get(3)?;
            let artist: String = row.get(4)?;

            println!("{} | {} | {} | {} | {}", number, name, album, path, artist);
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

use gronk_types::Song;
use jwalk::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

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

fn fix(item: &str) -> String {
    item.replace(r"'", r"''")
}

pub struct Database {
    conn: Connection,
    busy: Arc<AtomicBool>,
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
                "CREATE TABLE song (
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
        }

        Ok(Self {
            conn: Connection::open(DB_DIR.as_path()).unwrap(),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    pub fn add_music(&self, music_dir: &str) {
        let music_dir = music_dir.to_string();
        let busy = self.busy.clone();
        busy.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            let paths: Vec<PathBuf> = WalkDir::new(music_dir)
                .into_iter()
                .filter_map(|entry| {
                    if let Some(ex) = entry.as_ref().unwrap().path().extension() {
                        if ex == "flac" || ex == "mp3" || ex == "m4a" {
                            return Some(entry.as_ref().unwrap().path());
                        }
                    }
                    None
                })
                .collect();

            let songs: Vec<Song> = paths.par_iter().map(|path| Song::from(path)).collect();

            let stmts:Vec<_> = songs.iter().map(|song| {
                let artist = fix(&song.artist);
                let album = fix(&song.album);
                let name = fix(&song.name);
                let path = fix(&song.path.to_str().unwrap());
                format!("INSERT INTO song (number, disc, name, album, artist, path) VALUES ('{}', '{}', '{}', '{}', '{}', '{}');",
                            song.number, song.disc, name, album, artist, path)
            }).collect();

            let mut stmt = stmts.join("\n");

            stmt.insert_str(0, "BEGIN;\n");
            stmt.push_str("COMMIT;\n");

            let conn = Connection::open(DB_DIR.as_path()).unwrap();

            conn.execute_batch(&stmt).unwrap();

            //slow down to make sure the app has time to update
            thread::sleep(Duration::from_millis(16));
            busy.store(false, Ordering::SeqCst);
        });
    }
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }
    pub fn add_dir(&self, music_dir: &str) {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO music (path) VALUES (?1)",
                params![music_dir],
            )
            .unwrap();
        self.add_music(music_dir);
    }
    pub fn reset(&self) {
        self.conn.execute("DELETE FROM song", []).unwrap();
        let mut stmt = self.conn.prepare("SELECT path FROM music").unwrap();
        let paths: Vec<String> = stmt
            .query_map([], |row| Ok(row.get(0).unwrap()))
            .unwrap()
            .flatten()
            .collect();

        for path in paths {
            self.add_music(&path);
        }
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
    pub fn artists(&self) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")
            .unwrap();

        let mut rows = stmt.query([]).unwrap();

        let mut artists = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let artist: String = row.get(0).unwrap();
            artists.push(artist);
        }
        artists
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
    pub fn albums_by_artist(&self, artist: &str) -> Vec<String> {
        let artist = fix(artist);

        let query = format!(
            "SELECT DISTINCT album FROM song WHERE artist = '{}' ORDER BY album COLLATE NOCASE",
            artist
        );

        let mut stmt = self.conn.prepare(&query).unwrap();
        let mut rows = stmt.query([]).unwrap();

        let mut albums = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let album: String = row.get(0).unwrap();
            albums.push(album);
        }

        albums
    }
    pub fn songs_from_album(&self, artist: &str, album: &str) -> Vec<(u16, String)> {
        let artist = fix(artist);
        let album = fix(album);

        let query = format!(
            "SELECT number, name FROM song WHERE artist='{}' AND album='{}' ORDER BY disc, number",
            artist, album
        );

        let mut stmt = self.conn.prepare(&query).unwrap();
        let mut rows = stmt.query([]).unwrap();

        let mut songs = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let number: u16 = row.get(0).unwrap();
            let name: String = row.get(1).unwrap();
            songs.push((number, name));
        }

        songs
    }
    pub fn get_artist(&self, artist: &str) -> Vec<Song> {
        let artist = fix(artist);

        let query = format!(
            "SELECT * FROM song WHERE artist = '{}' ORDER BY album, disc, number",
            artist,
        );

        self.collect_songs(&query)
    }
    pub fn get_album(&self, artist: &str, album: &str) -> Vec<Song> {
        let artist = fix(artist);
        let album = fix(album);

        let query = format!(
            "SELECT * FROM song WHERE artist='{}' AND album='{}' ORDER BY disc, number",
            artist, album
        );

        self.collect_songs(&query)
    }
    pub fn get_song(&self, artist: &str, album: &str, song: &(u16, String)) -> Vec<Song> {
        let artist = fix(artist);
        let album = fix(album);
        let name = fix(&song.1);

        let query = format!(
            "SELECT * FROM song WHERE name='{}' AND number='{}' AND artist='{}' AND album='{}'",
            name, song.0, artist, album
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

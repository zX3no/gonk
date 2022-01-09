use dpc_pariter::IteratorExt;
use gronk_types::Song;
use jwalk::WalkDir;
use rusqlite::{params, Connection, Row};
use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, SyncSender},
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
    tx: Arc<SyncSender<bool>>,
    rx: Receiver<bool>,
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
                    number   INTEGER NOT NULL,
                    disc     INTEGER NOT NULL,
                    name     TEXT NOT NULL,
                    album    TEXT NOT NULL,
                    artist   TEXT NOT NULL,
                    path     TEXT NOT NULL,
                    duration DOUBLE NOT NULL
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

        let (tx, rx) = mpsc::sync_channel(1);
        Ok(Self {
            conn: Connection::open(DB_DIR.as_path()).unwrap(),
            tx: Arc::new(tx),
            rx,
        })
    }
    pub fn add_music(&self, music_dir: &str) {
        let music_dir = music_dir.to_string();
        let tx = self.tx.clone();

        thread::spawn(move || {
            let mut stmt: String = WalkDir::new(music_dir)
                .into_iter()
                .map(|dir| dir.unwrap().path())
                .filter(|dir| {
                    if let Some(ex) = dir.extension() {
                        matches!(ex.to_str(), Some("flac") | Some("mp3"))
                    } else {
                        false
                    }
                })
                .parallel_map(|dir| Song::from(&dir))
                .map(|song| {
                    let artist = fix(&song.artist);
                    let album = fix(&song.album);
                    let name = fix(&song.name);
                    let path = fix(song.path.to_str().unwrap());
                    format!("INSERT INTO song (number, disc, name, album, artist, path, duration) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.duration.as_secs_f64())
                })
                .collect::<Vec<_>>().join("\n");

            stmt.insert_str(0, "BEGIN;\n");
            stmt.push_str("COMMIT;\n");

            let conn = Connection::open(DB_DIR.as_path()).unwrap();

            conn.execute_batch(&stmt).unwrap();

            tx.send(true).unwrap();
        });
    }
    pub fn is_busy(&self) -> bool {
        if let Ok(recv) = self.rx.try_recv() {
            recv
        } else {
            false
        }
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
                songs.push(Database::song(row));
            }
        }
        songs
    }
    pub fn get_song_from_id(&self, id: usize) -> Song {
        let query = format!("SELECT * FROM song WHERE rowid='{}'", id);
        let mut stmt = self.conn.prepare(&query).unwrap();
        let mut rows = stmt.query([]).unwrap();

        if let Some(row) = rows.next().unwrap() {
            Database::song(row)
        } else {
            panic!();
        }
    }
    pub fn get_songs(&self) -> Vec<(Song, usize)> {
        let mut stmt = self.conn.prepare("SELECT *, rowid FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(7).unwrap();
            let song = Database::song(row);
            Ok((song, id))
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

        stmt.query_map([], |row| Ok(Database::song(row)))
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
        }
    }
}

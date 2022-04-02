use crate::{CONFIG_DIR, DB_DIR};
use dpc_pariter::IteratorExt;
use gonk_types::Song;
use jwalk::WalkDir;
use rusqlite::{params, Connection, Params, Row};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

fn fix(item: &str) -> String {
    item.replace('\'', r"''")
}

pub struct Database {
    conn: Connection,
    busy: Arc<AtomicBool>,
}

//TODO: fix function names, they don't follow any convention.
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
                    path     TEXT NOT NULL UNIQUE,
                    duration DOUBLE NOT NULL,
                    parent   TEXT NOT NULL
                )",
                [],
            )?;
        }

        Ok(Self {
            conn: Connection::open(DB_DIR.as_path()).unwrap(),
            busy: Arc::new(AtomicBool::new(false)),
        })
    }
    //TODO: this is super overcomplicated
    pub fn add(&self, toml_paths: &[String]) {
        let db_paths = self.get_paths();

        //add paths that are in toml file but not the database
        for t_path in toml_paths {
            if !db_paths.contains(t_path) {
                self.force_add(t_path);
            }
        }

        //delete paths that aren't in the toml file but are in the database
        for db_path in db_paths {
            if !toml_paths.contains(&db_path) {
                self.delete_path(&db_path);
            }
        }
    }
    pub fn force_add(&self, dir: &str) {
        let dir = dir.to_owned();
        let busy = self.busy.clone();
        busy.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            let songs: Vec<Song> = WalkDir::new(&dir)
                .into_iter()
                .map(|dir| dir.unwrap().path())
                .filter(|dir| {
                    if let Some(ex) = dir.extension() {
                        matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
                    } else {
                        false
                    }
                })
                .parallel_map(|dir| Song::from(&dir))
                .collect();

            if songs.is_empty() {
                return busy.store(false, Ordering::SeqCst);
            }

            let mut stmt = String::from("BEGIN;\n");
            stmt.push_str(&songs.iter()
                .map(|song| {
                    let artist = fix(&song.artist);
                    let album = fix(&song.album);
                    let name = fix(&song.name);
                    let path = fix(song.path.to_str().unwrap());
                    let parent = fix(&dir);
                    //TODO: would be nice to have batch params, don't think it's implemented.
                    format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, duration, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.duration.as_secs_f64(), parent)
                })
                .collect::<Vec<_>>().join("\n"));

            stmt.push_str("COMMIT;\n");

            let conn = Connection::open(DB_DIR.as_path()).unwrap();

            conn.execute_batch(&stmt).unwrap();

            busy.store(false, Ordering::SeqCst);
        });
    }
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }
    pub fn delete_path(&self, path: &str) {
        self.conn
            .execute("DELETE FROM song WHERE parent = ?", [path])
            .unwrap();
    }
    pub fn get_paths(&self) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT parent FROM song")
            .unwrap();

        stmt.query_map([], |row| Ok(row.get(0).unwrap()))
            .unwrap()
            .flatten()
            .collect()
    }
    pub fn get_songs_from_ids(&self, ids: &[usize]) -> Vec<Song> {
        if ids.is_empty() {
            return Vec::new();
        }

        let mut query = format!("SELECT * FROM song WHERE rowid={}", ids.first().unwrap());

        ids.iter()
            .skip(1)
            .for_each(|id| query.push_str(&format!(" OR rowid={}", id)));

        let mut stmt = self.conn.prepare(&query).unwrap();
        stmt.query_map([], |row| Ok(Database::song(row)))
            .unwrap()
            .flatten()
            .collect()
    }
    pub fn get_song_from_id(&self, id: usize) -> Song {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM song WHERE rowid=?")
            .unwrap();
        stmt.query_row([id], |row| Ok(Database::song(row))).unwrap()
    }
    pub fn get_songs(&self) -> Vec<(usize, Song)> {
        let mut stmt = self.conn.prepare("SELECT *, rowid FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(8).unwrap();
            let song = Database::song(row);
            Ok((id, song))
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

        stmt.query_map([], |row| {
            let artist: String = row.get(0).unwrap();
            Ok(artist)
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn albums(&self) -> Vec<(String, String)> {
        let mut stmt = self
            .conn
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
    pub fn albums_by_artist(&self, artist: &str) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT album FROM song WHERE artist = ? ORDER BY album COLLATE NOCASE",
            )
            .unwrap();

        stmt.query_map([artist], |row| {
            let album: String = row.get(0).unwrap();
            Ok(album)
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn songs_from_album(&self, album: &str, artist: &str) -> Vec<(u16, String)> {
        let mut stmt = self.conn.prepare("SELECT number, name FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number").unwrap();

        stmt.query_map([artist, album], |row| {
            let number: u16 = row.get(0).unwrap();
            let name: String = row.get(1).unwrap();
            Ok((number, name))
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn artist(&self, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE artist = ? ORDER BY album, disc, number",
            params![artist],
        )
    }
    pub fn album(&self, album: &str, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number",
            params![artist, album],
        )
    }
    pub fn get_song(&self, song: &(u16, String), album: &str, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE name=(?1) AND number=(?2) AND artist=(?3) AND album=(?4)",
            params![song.1, song.0, artist, album],
        )
    }
    fn collect_songs<P>(&self, query: &str, params: P) -> Vec<Song>
    where
        P: Params,
    {
        let mut stmt = self.conn.prepare(query).unwrap();

        stmt.query_map(params, |row| Ok(Database::song(row)))
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
    pub fn delete() {
        std::fs::remove_file(DB_DIR.as_path()).unwrap();
    }
}

use crate::{Song, DB_DIR};
use dpc_pariter::IteratorExt;
use jwalk::WalkDir;
use rusqlite::{params, Connection, Params, Row};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

fn fix(item: &str) -> String {
    item.replace('\'', r"''")
}

pub struct Database {
    conn: Mutex<Connection>,
    is_busy: Arc<AtomicBool>,
    needs_update: Arc<AtomicBool>,
}

impl Database {
    pub fn new() -> rusqlite::Result<Self> {
        if !DB_DIR.exists() {
            let conn = Connection::open(DB_DIR.as_path()).unwrap();
            conn.busy_timeout(Duration::from_millis(0))?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "synchronous", "0")?;
            conn.pragma_update(None, "temp_store", "MEMORY")?;

            //TODO: store the volume in the database
            conn.execute(
                "CREATE TABLE song (
                    number   INTEGER NOT NULL,
                    disc     INTEGER NOT NULL,
                    name     TEXT NOT NULL,
                    album    TEXT NOT NULL,
                    artist   TEXT NOT NULL,
                    path     TEXT NOT NULL UNIQUE,
                    track_gain DOUBLE NOT NULL,
                    parent   TEXT NOT NULL
                )",
                [],
            )?;
        }

        Ok(Self {
            conn: Mutex::new(Connection::open(DB_DIR.as_path()).unwrap()),
            needs_update: Arc::new(AtomicBool::new(false)),
            is_busy: Arc::new(AtomicBool::new(false)),
        })
    }
    fn conn(&self) -> MutexGuard<Connection> {
        self.conn.lock().unwrap()
    }
    pub fn needs_update(&self) -> bool {
        self.needs_update.load(Ordering::Relaxed)
    }
    pub fn stop(&mut self) {
        self.needs_update.store(false, Ordering::SeqCst)
    }
    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::SeqCst)
    }
    pub fn sync_database(&self, toml_paths: &[String]) {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT DISTINCT parent FROM song").unwrap();
        let paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect();

        //delete paths that aren't in the toml file but are in the database
        for path in &paths {
            if !toml_paths.contains(path) {
                println!("Removing: {path}");
                conn.execute("DELETE FROM song WHERE parent = ?", [path])
                    .unwrap();
            }
        }

        //find the paths that are missing from the database
        let paths_to_add: Vec<_> = toml_paths
            .iter()
            .filter_map(|path| {
                if !paths.contains(path) {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        if !paths_to_add.is_empty() {
            println!("Adding: {:?}", paths_to_add);
            self.add_paths(&paths_to_add);
        }
    }
    pub fn add_paths(&self, paths: &[String]) {
        if self.is_busy() {
            return;
        }

        let busy = self.is_busy.clone();
        let update = self.needs_update.clone();
        let dirs = paths.to_owned();
        busy.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            for dir in dirs {
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
                    return;
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
                    format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, track_gain, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.track_gain, parent)
                })
                .collect::<Vec<_>>().join("\n"));

                stmt.push_str("COMMIT;\n");

                let conn = Connection::open(DB_DIR.as_path()).unwrap();
                conn.execute_batch(&stmt).unwrap();
            }

            busy.store(false, Ordering::SeqCst);
            update.store(true, Ordering::SeqCst);
        });
    }
    pub fn get_songs_from_id(&self, ids: &[u64]) -> Vec<Song> {
        ids.iter()
            .filter_map(|id| {
                self.collect_songs("SELECT rowid, * FROM song WHERE rowid = ?", params![id])
                    .first()
                    .cloned()
            })
            .collect()
    }
    pub fn get_all_songs(&self) -> Vec<(usize, Song)> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT *, rowid FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(9).unwrap();
            let song = Database::song(row);
            Ok((id, song))
        })
        .unwrap()
        .flatten()
        .collect()
    }
    pub fn get_all_artists(&self) -> Vec<String> {
        let conn = self.conn();
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
    pub fn get_all_albums(&self) -> Vec<(String, String)> {
        let conn = self.conn();
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
    pub fn get_all_albums_by_artist(&self, artist: &str) -> Vec<String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT album FROM song WHERE artist = ? ORDER BY album COLLATE NOCASE",
            )
            .unwrap();

        stmt.query_map([artist], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect()
    }
    pub fn get_songs_by_artist(&self, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT rowid, * FROM song WHERE artist = ? ORDER BY album, disc, number",
            params![artist],
        )
    }
    pub fn get_songs_from_album(&self, album: &str, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT rowid, * FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number",
            params![artist, album],
        )
    }
    pub fn get_song(&self, song: &(u64, String), album: &str, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT rowid, * FROM song WHERE name=(?1) AND number=(?2) AND artist=(?3) AND album=(?4)",
            params![song.1, song.0, artist, album],
        )
    }
    fn collect_songs<P>(&self, query: &str, params: P) -> Vec<Song>
    where
        P: Params,
    {
        let conn = self.conn();
        let mut stmt = conn.prepare(query).unwrap();

        stmt.query_map(params, |row| Ok(Database::song(row)))
            .unwrap()
            .flatten()
            .collect()
    }
    fn song(row: &Row) -> Song {
        let path: String = row.get(6).unwrap();
        Song {
            id: Some(row.get(0).unwrap()),
            number: row.get(1).unwrap(),
            disc: row.get(2).unwrap(),
            name: row.get(3).unwrap(),
            album: row.get(4).unwrap(),
            artist: row.get(5).unwrap(),
            path: PathBuf::from(path),
            track_gain: row.get(7).unwrap(),
        }
    }
    pub fn delete() {
        if DB_DIR.as_path().exists() {
            std::fs::remove_file(DB_DIR.as_path()).unwrap();
        }
    }
}

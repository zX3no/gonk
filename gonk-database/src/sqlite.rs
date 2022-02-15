use crate::{CONFIG_DIR, DB_DIR};
use dpc_pariter::IteratorExt;
use gonk_types::Song;
use jwalk::WalkDir;
use rusqlite::{params, Connection, Params, Row};
use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, SyncSender},
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
                    path     TEXT NOT NULL UNIQUE,
                    duration DOUBLE NOT NULL,
                    parent   TEXT NOT NULL
                )",
                [],
            )?;
        }

        let (tx, rx) = mpsc::sync_channel(1);
        Ok(Self {
            conn: Connection::open(DB_DIR.as_path()).unwrap(),
            tx: Arc::new(tx),
            rx,
        })
    }
    pub fn add_music(&self, dirs: &[String]) {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT parent FROM song")
            .unwrap();

        let paths: Vec<String> = stmt
            .query_map([], |row| {
                let path: String = row.get(0).unwrap();

                //delete all of the old directories
                if !dirs.contains(&path) {
                    self.delete_path(&path);
                }
                Ok(path)
            })
            .unwrap()
            .flatten()
            .collect();

        //add all missing directories
        dirs.iter().for_each(|dir| {
            if !paths.contains(dir) {
                self.add(dir);
            }
        });
    }
    pub fn add(&self, music_dir: &str) {
        let music_dir = music_dir.to_string();
        let tx = self.tx.clone();
        tx.send(true).unwrap();

        thread::spawn(move || {
            let songs: Vec<Song> = WalkDir::new(&music_dir)
                .into_iter()
                .map(|dir| dir.unwrap().path())
                .filter(|dir| {
                    if let Some(ex) = dir.extension() {
                        matches!(
                            ex.to_str(),
                            Some("flac") | Some("mp3") | Some("ogg") | Some("wav") | Some("m4a")
                        )
                    } else {
                        false
                    }
                })
                .parallel_map(|dir| Song::from(&dir))
                .collect();

            if songs.is_empty() {
                panic!("Directory has no songs!");
            }

            let mut stmt = songs.iter()
                .map(|song| {
                    let artist = fix(&song.artist);
                    let album = fix(&song.album);
                    let name = fix(&song.name);
                    let path = fix(song.path.to_str().unwrap());
                    let parent = fix(&music_dir);
                    //TODO: would be nice to have batch params, don't think it's implemented.
                    format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, duration, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.duration.as_secs_f64(), parent)
                })
                .collect::<Vec<_>>().join("\n");

            stmt.insert_str(0, "BEGIN;\n");
            stmt.push_str("COMMIT;\n");

            let conn = Connection::open(DB_DIR.as_path()).unwrap();

            conn.execute_batch(&stmt).unwrap();

            tx.send(true).unwrap();
            tx.send(false).unwrap();
        });
    }
    pub fn is_busy(&self) -> Option<bool> {
        if let Ok(recv) = self.rx.try_recv() {
            Some(recv)
        } else {
            None
        }
    }
    pub fn delete_path(&self, path: &str) {
        self.conn
            .execute("DELETE FROM song WHERE parent = ?", [path])
            .unwrap();
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
    pub fn get_songs(&self) -> Vec<(Song, usize)> {
        let mut stmt = self.conn.prepare("SELECT *, rowid FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(8).unwrap();
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
    pub fn songs_from_album(&self, artist: &str, album: &str) -> Vec<(u16, String)> {
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
    pub fn get_artist(&self, artist: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE artist = ? ORDER BY album, disc, number",
            params![artist],
        )
    }
    pub fn get_album(&self, artist: &str, album: &str) -> Vec<Song> {
        self.collect_songs(
            "SELECT * FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number",
            params![artist, album],
        )
    }
    pub fn get_song(&self, artist: &str, album: &str, song: &(u16, String)) -> Vec<Song> {
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

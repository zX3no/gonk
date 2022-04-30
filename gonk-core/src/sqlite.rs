use crate::{Song, DB_DIR};
use dpc_pariter::IteratorExt;
use jwalk::WalkDir;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Params, Row};
use static_init::dynamic;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

#[dynamic]
static CONN: Pool<SqliteConnectionManager> = {
    let exists = DB_DIR.exists();
    let manager = SqliteConnectionManager::file(DB_DIR.as_path()).with_init(|c| {
        c.execute_batch("PRAGMA journal_mode=WAL;PRAGMA synchronous=0;PRAGMA temp_store=MEMORY")
    });

    let pool = r2d2::Pool::new(manager).unwrap();

    if !exists {
        let conn = pool.get().unwrap();
        conn.execute(
            "CREATE TABLE song (
                    number   INTEGER NOT NULL,
                    disc     INTEGER NOT NULL,
                    name     TEXT NOT NULL,
                    album    TEXT NOT NULL,
                    artist   TEXT NOT NULL,
                    path     TEXT NOT NULL UNIQUE,
                    duration DOUBLE NOT NULL,
                    track_gain DOUBLE NOT NULL,
                    parent   TEXT NOT NULL
                )",
            [],
        )
        .unwrap();
    }
    pool
};

fn conn() -> PooledConnection<SqliteConnectionManager> {
    CONN.get().unwrap()
}
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
pub fn get_songs_by_artist(artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT * FROM song WHERE artist = ? ORDER BY album, disc, number",
        params![artist],
    )
}
pub fn get_songs_from_album(album: &str, artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT * FROM song WHERE artist=(?1) AND album=(?2) ORDER BY disc, number",
        params![artist, album],
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
fn fix(item: &str) -> String {
    item.replace('\'', r"''")
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
pub fn reset() {
    let conn = conn();
    conn.execute("DELETE FROM song", []).unwrap();
}

#[derive(Default)]
pub struct Database {
    is_busy: Arc<AtomicBool>,
    needs_update: Arc<AtomicBool>,
}

impl Database {
    pub fn sync_database(&self, toml_paths: &[String]) {
        let conn = conn();
        let mut stmt = conn.prepare("SELECT DISTINCT parent FROM song").unwrap();

        let paths: Vec<_> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .flatten()
            .collect();

        //delete paths that aren't in the toml file but are in the database
        paths.iter().for_each(|path| {
            if !toml_paths.contains(path) {
                conn.execute("DELETE FROM song WHERE parent = ?", [path])
                    .unwrap();
            }
        });

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
            self.add_dirs(&paths_to_add);
        }
    }
    pub fn add_dirs(&self, dirs: &[String]) {
        if self.is_busy() {
            return;
        }

        let busy = self.is_busy.clone();
        let update = self.needs_update.clone();
        let dirs = dirs.to_owned();
        busy.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            for dir in dirs {
                let songs: Vec<Song> = WalkDir::new(&dir)
                    .into_iter()
                    .flatten()
                    .map(|dir| dir.path())
                    .filter(|path| match path.extension() {
                        Some(ex) => {
                            matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
                        }
                        None => false,
                    })
                    .parallel_map(|dir| Song::from(&dir).unwrap())
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
                    format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, duration, track_gain, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.duration.as_secs_f64(), song.track_gain, parent)
                })
                .collect::<Vec<_>>().join("\n"));

                stmt.push_str("COMMIT;\n");

                let conn = conn();
                conn.execute_batch(&stmt).unwrap();
            }

            busy.store(false, Ordering::SeqCst);
            update.store(true, Ordering::SeqCst);
        });
    }
    pub fn needs_update(&self) -> bool {
        self.needs_update.load(Ordering::Relaxed)
    }
    pub fn is_busy(&self) -> bool {
        self.is_busy.load(Ordering::SeqCst)
    }
    pub fn stop(&mut self) {
        self.needs_update.store(false, Ordering::SeqCst)
    }
}

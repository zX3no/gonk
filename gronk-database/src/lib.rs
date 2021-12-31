use gronk_types::Song;
use jwalk::WalkDir;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey},
    probe::Hint,
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

pub struct Database {
    conn: Connection,
    adding_songs: Arc<AtomicBool>,
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
            adding_songs: Arc::new(AtomicBool::new(false)),
        })
    }
    pub fn busy(&self) -> bool {
        self.adding_songs.load(Ordering::Relaxed)
    }
    pub fn add_music(&self, music_dir: &str) {
        let music_dir = music_dir.to_string();

        let adding_songs = self.adding_songs.clone();
        adding_songs.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let now = Instant::now();

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

            let sqlite_connection_manager = SqliteConnectionManager::file(DB_DIR.as_path());
            let sqlite_pool = Pool::new(sqlite_connection_manager).unwrap();

            paths.par_iter().for_each(|path| {
                let song = Database::song(path);
                let conn = sqlite_pool.get().unwrap();
                conn
                    .execute(
                        "INSERT INTO song (number, disc, name, album, artist, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![song.number, song.disc, song.name, song.album, song.artist, song.path.to_str().unwrap()],
                    ).unwrap();
            });

            eprintln!("total: {:?}", now.elapsed());
        });
    }
    pub fn song(path: &Path) -> Song {
        let mut hint = Hint::new();
        let ext = path.extension().unwrap().to_str().unwrap();
        hint.with_extension(&ext);

        let file = Box::new(File::open(path).unwrap());

        // Create the media source stream using the boxed media source from above.
        let mss = MediaSourceStream::new(file, Default::default());

        // Use the default options for metadata and format readers.
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();

        let mut probe = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .unwrap();

        let mut song = Song::default();
        song.path = path.to_path_buf();

        let mut get_songs = |metadata: &MetadataRevision| {
            for tag in metadata.tags() {
                if let Some(std_key) = tag.std_key {
                    if let StandardTagKey::AlbumArtist = std_key {
                        song.artist = tag.value.to_string();
                    } else if let StandardTagKey::Album = std_key {
                        song.album = tag.value.to_string();
                    } else if let StandardTagKey::TrackTitle = std_key {
                        song.name = tag.value.to_string();
                    } else if let StandardTagKey::TrackNumber = std_key {
                        song.number = tag.value.to_string().parse::<u16>().unwrap_or(1);
                    } else if let StandardTagKey::DiscNumber = std_key {
                        song.disc = tag.value.to_string().parse::<u16>().unwrap_or(1);
                    }
                }
            }
        };

        if let Some(metadata) = probe.metadata.get() {
            let metadata = metadata.current().unwrap();
            get_songs(metadata);
        } else if let Some(metadata) = probe.format.metadata().current() {
            get_songs(metadata);
        }
        song
        // dbg!(song)
    }
    pub fn fast_add_music(&self, music_dir: &str) {
        let music_dir = music_dir.to_string();

        let adding_songs = self.adding_songs.clone();
        adding_songs.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let total = Instant::now();

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

            eprintln!("paths: {:?}", total.elapsed());
            let now = Instant::now();

            let songs: Vec<Song> = paths.par_iter().map(|path| Database::song(path)).collect();

            eprintln!("songs: {:?}", now.elapsed());
            let now = Instant::now();

            let sqlite_connection_manager = SqliteConnectionManager::file(DB_DIR.as_path());
            let sqlite_pool = Pool::new(sqlite_connection_manager).unwrap();

            songs.par_iter().for_each(|song| {
                    let connection = sqlite_pool.get().unwrap();
                    connection
                        .execute(
                            "INSERT INTO song (number, disc, name, album, artist, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![song.number, song.disc, song.name, song.album, song.artist, song.path.to_str().unwrap()],
                        ).unwrap();
                });

            eprintln!("database: {:?}", now.elapsed());
            eprintln!("total: {:?}", total.elapsed());
            adding_songs.store(false, Ordering::Relaxed);
        });
    }
    pub fn add_dir(&self, music_dir: &str) {
        let conn = Connection::open(DB_DIR.as_path()).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO music (path) VALUES (?1)",
            params![music_dir],
        )
        .unwrap();
        self.fast_add_music(music_dir);
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
fn fix(item: &str) -> String {
    item.replace("\'", "\'\'")
}

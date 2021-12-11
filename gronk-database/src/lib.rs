use gronk_types::Song;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{params, Connection, Result};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use walkdir::WalkDir;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(music_dirs: Vec<&Path>) -> rusqlite::Result<Self> {
        let config = dirs::config_dir().unwrap();
        let dir = format!("{}\\gronk", config.to_string_lossy());

        if !Path::new(&dir).exists() {
            std::fs::create_dir(&dir).unwrap();
        }

        let db = format!("{}\\music.db", dir);

        if !Path::new(&db).exists() {
            let conn = Connection::open(&db).unwrap();
            conn.busy_timeout(Duration::from_millis(0))?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "synchronous", "0")?;
            conn.pragma_update(None, "temp_store", "MEMORY")?;

            conn.execute(
                "CREATE VIRTUAL TABLE song USING FTS4(
                    number  INTEGER NOT NULL,
                    disc    INTEGER NOT NULL,
                    name    TEXT NOT NULL,
                    album   TEXT NOT NULL,
                    artist  TEXT NOT NULL,
                    path    TEXT NOT NULL
                )",
                [],
            )?;

            for dir in music_dirs {
                let paths: Vec<PathBuf> = WalkDir::new(dir)
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

                let songs: Vec<Song> = paths
                    .par_iter()
                    .map(|path| Song::from(path.to_str().unwrap()))
                    .collect();

                let sqlite_connection_manager = SqliteConnectionManager::file(&db);
                let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager).unwrap();

                let pool_arc = Arc::new(sqlite_pool);

                songs.par_iter().for_each(|song| {
                    let connection = pool_arc.get().unwrap();
                    connection
                        .execute(
                            "INSERT INTO song (number, disc, name, album, artist, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![song.number, song.disc, song.name, song.album, song.artist, song.path.to_str().unwrap()],
                        ).unwrap();
                });
            }

            conn.execute("INSERT INTO song(song) VALUES('optimize')", [])?;
        }

        Ok(Self {
            conn: Connection::open(&db).unwrap(),
        })
    }
    pub fn reset(&self, music_dirs: Vec<&Path>) {
        let config = dirs::config_dir().unwrap();
        let db = format!("{}\\gronk\\music.db", config.to_string_lossy());
        File::create(db).unwrap();
        Database::new(music_dirs).unwrap();
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
            return Song {
                number: row.get(0).unwrap(),
                disc: row.get(1).unwrap(),
                name: row.get(2).unwrap(),
                album: row.get(3).unwrap(),
                artist: row.get(4).unwrap(),
                path: PathBuf::from(path),
            };
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
    pub fn search(&self, search: String) -> Vec<Song> {
        let query = if !search.is_empty() {
            let mut search = search.replace("\'", "\'\'");
            while search.ends_with(' ') {
                search.pop();
            }
            format!(
                "SELECT * FROM song WHERE song MATCH '\"{}*\"' ORDER BY artist, album, disc, number COLLATE NOCASE",
              search,
            )
        } else {
            "SELECT * FROM song ORDER BY artist, album, disc, number COLLATE NOCASE".to_string()
        };
        self.collect_songs(&query)
    }
    pub fn artists(&self) -> Result<Vec<String>> {
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
    pub fn albums_by_artist(&self, artist: &str) -> Result<Vec<String>> {
        let artist = fix(artist);

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
    pub fn songs_from_album(&self, artist: &str, album: &str) -> Result<Vec<(u16, String)>> {
        let artist = fix(artist);
        let album = fix(album);

        let query = format!(
            "SELECT number, name FROM song WHERE song MATCH 'artist:{} AND album:{}' ORDER BY disc, number",
            artist, album
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        let mut songs = Vec::new();
        while let Some(row) = rows.next()? {
            let number: u16 = row.get(0)?;
            let name: String = row.get(1)?;
            songs.push((number, name));
        }

        Ok(songs)
    }
    pub fn get_artist(&self, artist: &str) -> Vec<Song> {
        let artist = fix(artist);

        let query = format!(
            "SELECT * FROM song WHERE artist = '{}' ORDER BY album, disc, number",
            artist,
        );

        self.collect_songs(&query)
    }
    pub fn get_album(&self, album: &str, artist: &str) -> Vec<Song> {
        let album = fix(album);
        let artist = fix(artist);

        let query = format!(
            "SELECT * FROM song WHERE song MATCH 'album:{} AND artist:{}' ORDER BY disc, number",
            album, artist
        );

        self.collect_songs(&query)
    }
    pub fn get_song(&self, artist: &str, album: &str, number: &u16, name: &str) -> Vec<Song> {
        let artist = fix(artist);
        let album = fix(album);
        let name = fix(name);

        //TODO: benchmark queries and swap to fts4 for others
        //TODO: Disc number too?
        let query = format!(
            "SELECT * FROM song WHERE song MATCH 'name:{} AND number:{} AND artist:{} AND album:{}'",
            name, number, artist, album
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
    item.replace("\'", "\'\'").replace(":", "\\:").to_string()
}

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
    pub fn new() -> Self {
        if !Path::new("music.db").exists() {
            Database::create_db().unwrap();
        }
        Self {
            conn: Connection::open("music.db").unwrap(),
        }
    }
    pub fn create_db() -> rusqlite::Result<()> {
        let conn = Connection::open("music.db").unwrap();

        File::create("music.db").unwrap();
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

        let paths: Vec<PathBuf> = WalkDir::new("D:/Music")
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

        //TODO: there is definatly a better way to do this
        let sqlite_connection_manager = SqliteConnectionManager::file("music.db");
        let sqlite_pool = r2d2::Pool::new(sqlite_connection_manager).unwrap();

        let pool_arc = Arc::new(sqlite_pool);

        songs.par_iter().for_each(|song| {
            let connection = pool_arc.get().unwrap();

            connection
                .execute(
                    "INSERT INTO song (number, disc, name, album, artist, path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![song.number, song.disc, song.name, song.album, song.artist, song.path.to_str().unwrap()],
                )
                .unwrap();
        });

        //idk if this does shit?
        conn.execute("INSERT INTO song(song) VALUES('optimize')", [])?;

        Ok(())
    }
    pub fn get_all_song_names(&self) -> Vec<String> {
        let mut stmt = self.conn.prepare("SELECT name FROM song").unwrap();
        let mut rows = stmt.query([]).unwrap();
        let mut songs = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let song: String = row.get(0).unwrap();
            songs.push(song);
        }
        songs
    }
    pub fn get_all_songs(&self) -> Vec<Song> {
        self.collect_songs("SELECT * FROM SONG ORDER BY artist, album, disc, number")
    }
    pub fn get_songs_from_ids(&self, ids: &Vec<usize>) -> Vec<Song> {
        // let mut query = String::from("SELECT * FROM song");

        // if ids.is_empty() {
        //     // self.collect_songs(&query)
        //     Vec::new()
        // } else {
        //     query.push_str(" where ");
        //     for id in ids {
        //         query.push_str(format!("rowid='{}' OR ", id).as_str());
        //     }
        //     let (query, _) = query.split_at(query.len() - 4);
        //     dbg!(&query);
        //     self.collect_songs(&query)
        // }
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
    pub fn get_all_ids(&self) -> Vec<(usize, String)> {
        let mut stmt = self.conn.prepare("SELECT rowid, name FROM song").unwrap();
        let mut rows = stmt.query([]).unwrap();
        let mut songs = Vec::new();

        while let Some(row) = rows.next().unwrap() {
            let id: usize = row.get(0).unwrap();
            let name: String = row.get(1).unwrap();
            songs.push((id, name));
        }
        songs
    }
    pub fn test(&self) -> Vec<(usize, Song)> {
        let mut stmt = self.conn.prepare("SELECT rowid, * FROM song").unwrap();

        stmt.query_map([], |row| {
            let id = row.get(0).unwrap();
            let path: String = row.get(6).unwrap();
            Ok((
                id,
                Song {
                    number: row.get(1).unwrap(),
                    disc: row.get(2).unwrap(),
                    name: row.get(3).unwrap(),
                    album: row.get(4).unwrap(),
                    artist: row.get(5).unwrap(),
                    path: PathBuf::from(path),
                },
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
    pub fn first_artist(&self) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let artist: String = row.get(0)?;
            return Ok(artist);
        }
        panic!("no artists");
    }
    pub fn first_album(&self, artist: &str) -> Result<String> {
        let query = format!(
            "SELECT DISTINCT album FROM song WHERE artist = '{}' ORDER BY album COLLATE NOCASE",
            artist
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let album: String = row.get(0)?;
            return Ok(album);
        }
        panic!("no albums");
    }
    pub fn first_song(&self, artist: &str, album: &str) -> Result<(u16, String)> {
        let query = format!(
            "SELECT number, name FROM song WHERE artist = '{}' AND album = '{}' ORDER BY disc, number",
            artist, album
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let track: u16 = row.get(0)?;
            let name: String = row.get(1)?;
            return Ok((track, name));
        }

        panic!("no albums");
    }
    pub fn get_artists(&self) -> Result<Vec<String>> {
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
    pub fn get_albums_by_artist(&self, artist: &str) -> Result<Vec<String>> {
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
    pub fn get_songs_from_album(&self, artist: &str, album: &str) -> Result<Vec<(u16, String)>> {
        // let query = format!(
        //     "SELECT number, name FROM song WHERE artist = '{}' AND album = '{}' ORDER BY disc, number",
        //     artist, album
        // );
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
        let query = format!(
            "SELECT * FROM song WHERE artist = '{}' ORDER BY album, disc, number",
            artist,
        );
        self.collect_songs(&query)
    }
    pub fn get_album(&self, artist: &str, album: &str) -> Vec<Song> {
        let query = format!(
            "SELECT * FROM song WHERE song MATCH 'artist:{} AND album:{}' ORDER BY disc, number",
            artist, album
        );

        self.collect_songs(&query)
    }
    pub fn get_song(&self, artist: &str, album: &str, number: &u16, name: &str) -> Vec<Song> {
        //this seems bad but it only takes like 2us
        let artist = artist.replace("\'", "\'\'");
        let album = album.replace("\'", "\'\'");
        let name = name.replace("\'", "\'\'");

        let artist = artist.replace(":", "\\:");
        let album = album.replace(":", "\\:");
        let name = name.replace(":", "\\:");

        //TODO: benchmark queries and swap to fts4 for others
        //TODO: Disc number too?
        let query = format!(
            "SELECT * FROM song WHERE song MATCH 'name:{} AND number:{} AND artist:{} AND album:{}'",
            name, number, artist, album
        );

        self.collect_songs(&query)
    }
    pub fn collect_songs(&self, query: &str) -> Vec<Song> {
        let mut stmt = self.conn.prepare(&query).unwrap();

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

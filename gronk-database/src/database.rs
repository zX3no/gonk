use std::{fs::File, time::Instant};

use rusqlite::{params, Connection, Result};
pub struct Database {
    conn: Connection,
}
impl Database {
    pub fn new() -> Self {
        Self {
            conn: Connection::open("music.db").unwrap(),
        }
    }
    pub fn create_db(&self) -> Result<()> {
        let conn = &self.conn;

        File::create("music.db").unwrap();
        conn.execute("PRAGMA foregin_keys = ON", [])?;

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
                    artist      TEXT NOT NULL,
                    FOREIGN     KEY(artist) REFERENCES artist(id)
                )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE song(
                    name    TEXT,
                    path    TEXT,
                    track   INTEGER,
                    album   TEXT NOT NULL REFERENCES album(id)
                )",
            [],
        )?;

        Ok(())
    }

    pub fn add_artist(&self, artist: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO artist(id) VALUES (?1)",
            params![artist],
        )?;

        Ok(())
    }
    pub fn add_album(&self, album: &str, artist: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO album(name, artist) VALUES (?1, ?2)",
            params![album, artist],
        )?;
        Ok(())
    }
    pub fn add_song(
        &self,
        track: usize,
        name: &str,
        album: &str,
        artist: &str,
        path: &str,
    ) -> Result<()> {
        self.add_artist(artist)?;
        self.add_album(album, artist)?;

        let query = format!("INSERT INTO song (track, name, path, album) VALUES (?1, ?2, ?3, (SELECT id FROM album WHERE name = \"{}\" AND artist = \"{}\"))", album, artist);
        self.conn.execute(&query, params![track, name, path])?;

        Ok(())
    }
}

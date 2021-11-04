use std::{fs::File, time::Instant};

use rusqlite::{Connection, Result};
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
        // conn.execute("PRAGMA journal_mode = MEMORY", [])?;
        conn.execute("PRAGMA synchronous = 0", [])?;
        // conn.execute("PRAGMA cache_size = 1000000", [])?;
        conn.execute("PRAGMA temp_store = MEMORY", [])?;

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

    pub fn add_song(
        &self,
        track: usize,
        name: &str,
        album: &str,
        artist: &str,
        path: &str,
    ) -> Result<()> {
        let now = Instant::now();
        //oh yeah this is what i'm talking about
        //2 seconds for all songs
        //in a thread pool it should be fast
        //i think sqlite was build for that
        let query = format!(
            "BEGIN; 
            INSERT OR IGNORE INTO artist(id) VALUES ('{}'); 
            INSERT OR IGNORE INTO album(name, artist) VALUES ('{}', '{}'); 
            INSERT INTO song (track, name, path, album) VALUES('{}','{}','{}', (SELECT id FROM album WHERE name = '{}' AND artist = '{}'));
            COMMIT;",
            artist, album, artist,
            track, name, path, album, artist,
        );
        self.conn.execute_batch(&query)?;
        println!("{:?}", now.elapsed());

        Ok(())
    }
}

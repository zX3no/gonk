#![allow(unused)]
use rusqlite::*;

#[derive(Debug)]
pub struct Artist {
    name: String,
}

#[derive(Debug)]
pub struct Album {
    name: String,
    artist: String,
}

#[derive(Debug)]
pub struct Folder {
    pub path: String,
}

#[derive(Debug, Default)]
pub struct Song {
    pub name: String,
    pub path: String,
    pub album: String,
    pub artist: String,
    pub folder: String,
}

#[must_use]
pub fn init() -> Result<()> {
    let _ = std::fs::remove_file("gonk.db");
    let conn = Connection::open("gonk.db")?;

    conn.execute(
        "CREATE TABLE folder (
            path TEXT PRIMARY KEY
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE artist (
            name TEXT PRIMARY KEY
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE album (
            name TEXT PRIMARY KEY,
            artist_id TEXT NOT NULL,
            FOREIGN KEY (artist_id) REFERENCES artist (name) 
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE song (
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            album_id TEXT NOT NULL,
            artist_id TEXT NOT NULL,
            folder_id TEXT NOT NULL,
            FOREIGN KEY (album_id) REFERENCES album (name),
            FOREIGN KEY (artist_id) REFERENCES artist (name),
            FOREIGN KEY (folder_id) REFERENCES folder (path) 
        )",
        [],
    )?;

    //Used for intersects
    //https://www.sqlitetutorial.net/sqlite-intersect/
    conn.execute(
        "CREATE TABLE temp_song (
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            album_id TEXT NOT NULL,
            artist_id TEXT NOT NULL,
            folder_id TEXT NOT NULL,
            FOREIGN KEY (album_id) REFERENCES album (name),
            FOREIGN KEY (artist_id) REFERENCES artist (name),
            FOREIGN KEY (folder_id) REFERENCES folder (path) 
        )",
        [],
    )?;

    Ok(())
}

pub fn collect_songs() {
    //TODO: Check if the path is in the database and if it is, don't read the metadata.
}

#[must_use]
pub fn rescan_folder(folder: &str) -> Result<()> {
    let conn = Connection::open("gonk.db").unwrap();
    //Make sure folder exists
    if conn
        .execute("INSERT INTO folder (path) VALUES (?1)", [folder])
        .is_err()
    {
        //Collect the songs
        let song = Song {
            name: String::from("Joe's Song"),
            album: String::from("Joe's Album"),
            artist: String::from("Joe"),
            path: String::new(),
            folder: folder.to_string(),
        };

        //Clean the temp table
        conn.execute("DELETE FROM temp_song", [])?;

        //Collect songs to compare
        add_temp(song)?;

        //Drop the difference
        let mut stmt = conn.execute(
            "DELETE FROM song WHERE rowid IN (SELECT rowid FROM song EXCEPT SELECT rowid FROM temp_song)",
            [],
        )?;
    };

    Ok(())
}

#[must_use]
pub fn add_folder(folder: &str) -> Result<()> {
    let conn = Connection::open("gonk.db").unwrap();
    conn.execute("INSERT OR IGNORE INTO folder (path) VALUES (?1)", [folder])?;

    //Collect songs
    let song = Song {
        name: String::from("Joe's Song"),
        album: String::from("Joe's Album"),
        artist: String::from("Joe"),
        path: String::new(),
        folder: folder.to_string(),
    };
    let song2 = Song {
        name: String::from("Joe's 2nd Song"),
        album: String::from("Joe's Album"),
        artist: String::from("Joe"),
        path: String::new(),
        folder: folder.to_string(),
    };

    //Add songs
    add_song(song)?;
    add_song(song2)?;

    Ok(())
}

#[must_use]
pub fn add_song(song: Song) -> Result<()> {
    let conn = Connection::open("gonk.db")?;

    conn.execute(
        "INSERT OR IGNORE INTO artist (name) VALUES (?1)",
        params![&song.artist],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO album (name, artist_id) VALUES (?1, ?2)",
        params![&song.album, &song.artist],
    )?;

    conn.execute(
        "INSERT INTO song (name, path, album_id, artist_id, folder_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &song.name,
            &song.path,
            &song.album,
            &song.artist,
            &song.folder
        ],
    )?;

    Ok(())
}
pub fn add_temp(song: Song) -> Result<()> {
    let conn = Connection::open("gonk.db")?;

    conn.execute(
        "INSERT OR IGNORE INTO artist (name) VALUES (?1)",
        params![&song.artist],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO album (name, artist_id) VALUES (?1, ?2)",
        params![&song.album, &song.artist],
    )?;

    conn.execute(
        "INSERT INTO temp_song (name, path, album_id, artist_id, folder_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &song.name,
            &song.path,
            &song.album,
            &song.artist,
            &song.folder
        ],
    )?;

    Ok(())
}

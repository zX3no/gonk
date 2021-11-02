use rusqlite::{params, Connection, Result};
use std::fs::File;

fn create_db(conn: &Connection) -> Result<()> {
    File::create("music.db").unwrap();
    conn.execute("PRAGMA foregin_keys = ON", [])?;

    conn.execute(
        "CREATE TABLE artist(
                    id TEXT PRIMARY KEY,
                    UNIQUE(id)
                )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE album(
                    id TEXT PRIMARY KEY,
                    artist      TEXT NOT NULL,
                    FOREIGN     KEY(artist) REFERENCES artist(id)
                )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE song(
                    name    TEXT,
                    album   TEXT NOT NULL,
                    FOREIGN KEY(album) REFERENCES album(id)
                )",
        [],
    )?;

    Ok(())
}

fn main() -> Result<()> {
    let conn = Connection::open("music.db")?;

    create_db(&conn)?;

    add_artist(&conn, "JPEGMAFIA")?;
    add_artist(&conn, "Iglooghost")?;
    add_album(&conn, "LP!", "JPEGMAFIA")?;
    add_album(&conn, "EP!", "JPEGMAFIA")?;
    add_song(&conn, "Panic Emoji", "EP!")?;
    add_song(&conn, "Panic Emoji", "LP!")?;

    dbg!(get_song_artist(&conn)?);
    dbg!(get_artists(&conn)?);

    Ok(())
}

pub fn get_song_artist(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT album.artist FROM song INNER JOIN album ON song.album = album.id ")
        .unwrap();
    let mut rows = stmt.query([])?;

    let mut artists: Vec<String> = Vec::new();
    while let Some(row) = rows.next()? {
        artists.push(row.get(0)?);
    }

    Ok(artists)
}
pub fn get_artists(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT * FROM artist")?;
    let mut rows = stmt.query([])?;

    let mut artists: Vec<String> = Vec::new();
    while let Some(row) = rows.next()? {
        artists.push(row.get(0)?);
    }

    Ok(artists)
}
pub fn add_artist(conn: &Connection, name: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO artist (id) VALUES (?1)",
        params![name],
    )?;

    Ok(())
}
pub fn add_album(conn: &Connection, name: &str, artist: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO album (id, artist) VALUES (?1, ?2)",
        params![name, artist],
    )?;
    Ok(())
}
pub fn add_song(conn: &Connection, name: &str, album: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO song (name, album) VALUES (?1, ?2)",
        params![name, album],
    )?;
    Ok(())
}

use rusqlite::{params, Connection, Result};
use std::fs::File;

fn create_db(conn: &Connection) -> Result<()> {
    File::create("music.db").unwrap();
    conn.execute("PRAGMA foregin_keys = ON", [])?;

    conn.execute(
        "CREATE TABLE artist(
                    id INTEGER PRIMARY KEY,
                    name TEXT,
                    UNIQUE(name)
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
                    album   TEXT NOT NULL,
                    path    TEXT NOT NULL,
                    track   INTEGER NOT NULL,
                    artist  TEXT NOT NULL
                )",
        //album   INTEGER NOT NULL,
        //FOREIGN KEY(album) REFERENCES album(id)
        [],
    )?;

    Ok(())
}

fn main() -> Result<()> {
    let conn = Connection::open("music.db")?;

    create_db(&conn)?;

    add_album(&conn, "LP!", "JPEGMAFIA")?;
    add_album(&conn, "EP!", "JPEGMAFIA")?;
    add_album(&conn, "NEO", "Iglooghost")?;

    add_song(&conn, "Alpha Emoji", "EP!")?;
    add_song(&conn, "Panic Emoji", "EP!")?;
    add_song(&conn, "TEST", "LP!")?;
    add_song(&conn, "sussusususus", "NEO")?;

    // get_all_songs(&conn)?;

    let mut stmt = conn.prepare("SELECT album FROM song WHERE artist = \"me\"")?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let a: String = row.get(0)?;
        dbg!(a);
    }
    // dbg!(get_song_artist(&conn)?);
    // dbg!(get_artists(&conn)?);

    Ok(())
}
pub fn get_all_songs(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT song.name, song.album, path, track, artist.name FROM song INNER JOIN album ON song.album = album.id INNER JOIN artist ON album.artist = artist.id")
        .unwrap();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let album: usize = row.get(1)?;
        let path: String = row.get(2)?;
        let number: usize = row.get(3)?;
        let artist: String = row.get(4)?;

        println!("{} | {} | {} | {} | {}", number, name, album, path, artist);
    }
    Ok(())
}

pub fn get_song_artist(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT album.artist FROM song INNER JOIN album ON song.album = album.id")
        .unwrap();
    let mut rows = stmt.query([])?;

    let mut artists: Vec<String> = Vec::new();
    while let Some(row) = rows.next()? {
        artists.push(row.get(0)?);
    }

    Ok(artists)
}
pub fn get_artists(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT * FROM artist")?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let a: usize = row.get(0)?;
        dbg!(a);
    }

    Ok(())
}
pub fn find_artist_id(conn: &Connection, name: &str) -> Option<usize> {
    let query = format!("SELECT id FROM artist where name = \"{}\"", name);
    if let Ok(mut stmt) = conn.prepare(&query) {
        let mut rows = stmt.query([]).unwrap();

        while let Some(row) = rows.next().unwrap() {
            let id: usize = row.get(0).unwrap();
            return Some(id);
        }
    }
    None
}
pub fn find_album_id(conn: &Connection, name: &str) -> Option<usize> {
    let query = format!("SELECT id FROM album where name = \"{}\"", name);
    if let Ok(mut stmt) = conn.prepare(&query) {
        let mut rows = stmt.query([]).unwrap();

        while let Some(row) = rows.next().unwrap() {
            let id: usize = row.get(0).unwrap();
            return Some(id);
        }
    }
    None
}
pub fn get_album_artist(conn: &Connection, name: &str) -> Option<String> {
    let query = format!("select album from artist where name = \"{}\"", name);
    if let Ok(mut stmt) = conn.prepare(&query) {
        let mut rows = stmt.query([]).unwrap();

        while let Some(row) = rows.next().unwrap() {
            let id: String = row.get(0).unwrap();
            return Some(id);
        }
    }
    None
}

pub fn add_artist(conn: &Connection, name: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO artist (name) VALUES (?1)",
        params![name],
    )?;

    Ok(())
}

pub fn add_album(conn: &Connection, name: &str, artist: &str) -> Result<()> {
    let id = if let Some(id) = find_artist_id(conn, artist) {
        id
    } else {
        add_artist(conn, artist)?;
        find_artist_id(conn, artist).unwrap()
    };
    conn.execute(
        "INSERT INTO album (name, artist) VALUES (?1, ?2)",
        params![name, id],
    )?;
    Ok(())
}

pub fn add_song(conn: &Connection, name: &str, album: &str) -> Result<()> {
    //when adding a song it is important to add it to the correct album
    //a song will contain the song artist and album name
    //so it should be easy to find the correct one
    //this does not represent the final data

    // let id = if let Some(id) = find_album_id(conn, album) {
    //     id
    // } else {
    //     let artist = get_album_artist(conn, album);
    //     add_album(conn, album, &artist.unwrap())?;
    //     find_artist_id(conn, album).unwrap()
    // };
    conn.execute(
        "INSERT INTO song (name, album, path, track, artist) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, album, "example/path", 1, "me"],
    )?;
    Ok(())
}

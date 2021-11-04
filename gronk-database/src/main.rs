use database::Database;
use rusqlite::Result;
use std::time::Instant;
mod database;

// Song Name | Track Number | Album | Artist | Year

//Get all artists
//SELECT name FROM artist
//Get all albums from artist
//SELECT name FROM artist WHERE artist = 15
//Get all songs from album
//SELECT name FROM songs WHERE album = 131

//Add a song
//INSERT INTO song (name, path, track, album, artist) VALUES (?1, ?2, ?3, ?4)

//Add an album

//Add an artist
//INSERT OR IGNORE INTO artist (name) VALUES (?1)

fn main() -> Result<()> {
    let database = Database::new();
    let now = Instant::now();
    database.create_db()?;
    println!("{:?}", now.elapsed());

    database.get_artists()?;
    // database.get_album_by_artist("JPEGMAFIA")?;
    // database.get_songs_from_album("Veteran", "JPEGMAFIA")?;

    Ok(())
}

use std::{sync::Arc, thread};

use database::Database;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{Connection, Result};
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
    // database.create_db()?;
    // Database::write();

    // database.add_song(1, "NAME", "ALBUM", "ARTIST", "PATH")?;

    // database.get_songs()?;
    // database.get_artists()?;
    database.get_album_by_artist("JPEGMAFIA")?;
    database.get_songs_from_album("LP!", "JPEGMAFIA")?;

    Ok(())
}

use database::Database;
use rusqlite::Result;
use std::time::Instant;
mod database;

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

use std::time::Instant;

use database::Database;
use rusqlite::Result;
mod database;

fn main() -> Result<()> {
    let database = Database::new();

    let now = Instant::now();
    database.create_db()?;
    println!("{:?}", now.elapsed());

    // database.get_artists()?;
    // database.get_albums_by_artist("JPEGMAFIA")?;
    // database.get_songs_from_album("The Powers That B", "Death Grips")?;

    Ok(())
}

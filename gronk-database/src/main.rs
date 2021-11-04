use database::Database;
use rusqlite::Result;
mod database;

fn main() -> Result<()> {
    let database = Database::new();

    // database.create_db()?;

    database.get_artists()?;
    database.get_album_by_artist("JPEGMAFIA")?;
    database.get_songs_from_album("Veteran", "JPEGMAFIA")?;

    Ok(())
}

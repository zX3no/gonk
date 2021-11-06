use database::Database;
use rusqlite::Result;
mod database;

fn main() -> Result<()> {
    let database = Database::new();

    // database.create_db()?;

    // database.get_artists()?;
    // database.get_albums_by_artist("JPEGMAFIA")?;
    database.get_songs_from_album("The Powers That B", "Death Grips")?;

    Ok(())
}

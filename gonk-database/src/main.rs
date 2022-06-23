use gonk_database::*;
use rusqlite::*;

fn main() -> Result<()> {
    init().unwrap();

    add_folder("D:/Test")?;
    rescan_folder("D:/Test")?;

    let conn = Connection::open("gonk.db")?;
    let mut stmt = conn.prepare("SELECT * FROM song")?;

    let songs = stmt.query_map([], |row| {
        Ok(Song {
            name: row.get(0)?,
            path: row.get(1)?,
            album: row.get(2)?,
            artist: row.get(3)?,
            folder: row.get(4)?,
        })
    })?;

    for song in songs {
        println!("Found {:?}", song.unwrap());
    }

    Ok(())
}

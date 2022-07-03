use crate::query::*;
use crate::*;

#[derive(Debug)]
pub struct PlaylistSong {
    pub path: PathBuf,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub id: usize,
}

pub fn add(playlist: &str, ids: &[usize]) {
    let songs = songs_from_ids(ids);

    if songs.is_empty() {
        panic!("Failed to add song ids: {:?}", ids);
    }

    let mut conn = conn();
    conn.execute(
        "INSERT OR IGNORE INTO playlist (name) VALUES (?1)",
        [playlist],
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT OR IGNORE INTO playlist_item (path, name, album, artist, playlist_id)
                VALUES (?, ?, ?, ?, ?)",
            )
            .unwrap();

        for song in songs {
            stmt.execute(params![
                &song.path.to_string_lossy(),
                &song.name,
                &song.album,
                &song.artist,
                &playlist,
            ])
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

//Only select playlists with songs in them
pub fn playlists() -> Vec<String> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT playlist_id FROM playlist_item")
        .unwrap();

    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .flatten()
        .collect()
}

pub fn get(playlist_name: &str) -> Vec<PlaylistSong> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT path, name, album, artist, rowid FROM playlist_item WHERE playlist_id = ?")
        .unwrap();

    stmt.query_map([playlist_name], |row| {
        let path: String = row.get(0).unwrap();

        Ok(PlaylistSong {
            path: PathBuf::from(path),
            name: row.get(1).unwrap(),
            album: row.get(2).unwrap(),
            artist: row.get(3).unwrap(),
            id: row.get(4).unwrap(),
        })
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn remove_id(id: usize) {
    conn()
        .execute("DELETE FROM playlist_item WHERE rowid = ?", [id])
        .unwrap();
}

pub fn remove(name: &str) {
    let conn = conn();
    conn.execute("DELETE FROM playlist_item WHERE playlist_id = ?", [name])
        .unwrap();

    conn.execute("DELETE FROM playlist WHERE name = ?", [name])
        .unwrap();
}

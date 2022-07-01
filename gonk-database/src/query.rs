use crate::conn;
use gonk_player::Song;
use rusqlite::*;
use std::path::PathBuf;

pub fn cache(ids: &[usize]) {
    let mut conn = conn();

    let tx = conn.transaction().unwrap();
    tx.execute("DELETE FROM persist", []).unwrap();
    {
        let mut stmt = tx
            .prepare_cached("INSERT INTO persist (song_id) VALUES (?)")
            .unwrap();

        for id in ids {
            stmt.execute([id]).unwrap();
        }
    }
    tx.commit().unwrap();
}

pub fn get_cache() -> Vec<Song> {
    collect_songs(
        "SELECT *, rowid FROM song WHERE rowid IN (SELECT song_id FROM persist)",
        [],
    )
}

pub fn volume() -> u16 {
    conn()
        .query_row("SELECT volume FROM settings", [], |row| row.get(0))
        .unwrap()
}

pub fn set_volume(vol: u16) {
    conn()
        .execute("UPDATE settings SET volume = ?", [vol])
        .unwrap();
}

pub fn folders() -> Vec<String> {
    let conn = conn();
    let mut stmt = conn.prepare("SELECT folder FROM folder").unwrap();

    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .flatten()
        .collect()
}

pub fn remove_folder(path: &str) -> Result<(), &str> {
    let path = path.replace("\\", "/");
    let conn = conn();

    conn.execute("DELETE FROM song WHERE folder = ?", [&path])
        .unwrap();

    let result = conn
        .execute("DELETE FROM folder WHERE folder = ?", [path])
        .unwrap();

    if result == 0 {
        Err("Invalid path.")
    } else {
        Ok(())
    }
}

pub fn total_songs() -> usize {
    conn()
        .query_row("SELECT COUNT(*) FROM song", [], |row| row.get(0))
        .unwrap()
}

pub fn songs() -> Vec<Song> {
    collect_songs("SELECT *, rowid FROM song", [])
}

pub fn artists() -> Vec<String> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT artist FROM song ORDER BY artist COLLATE NOCASE")
        .unwrap();

    stmt.query_map([], |row| {
        let artist: String = row.get(0).unwrap();
        Ok(artist)
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn albums() -> Vec<(String, String)> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT album, artist FROM song ORDER BY artist COLLATE NOCASE")
        .unwrap();

    stmt.query_map([], |row| {
        let album: String = row.get(0).unwrap();
        let artist: String = row.get(1).unwrap();
        Ok((album, artist))
    })
    .unwrap()
    .flatten()
    .collect()
}

pub fn albums_by_artist(artist: &str) -> Vec<String> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT album FROM song WHERE artist = ? ORDER BY album COLLATE NOCASE")
        .unwrap();

    stmt.query_map([artist], |row| row.get(0))
        .unwrap()
        .flatten()
        .collect()
}

pub fn songs_from_album(album: &str, artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT *, rowid FROM song WHERE artist = (?1) AND album = (?2) ORDER BY disc, number",
        params![artist, album],
    )
}

pub fn songs_by_artist(artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT *, rowid FROM song WHERE artist = ? ORDER BY album, disc, number",
        params![artist],
    )
}

pub fn songs_from_ids(ids: &[usize]) -> Vec<Song> {
    let conn = conn();

    let sql: Vec<String> = ids
        .iter()
        .map(|id| format!("SELECT *, rowid FROM song WHERE rowid = {}\nUNION ALL", id))
        .collect();

    let sql = sql.join("\n");
    //Remove the last 'UNION ALL'
    let sql = &sql[..sql.len() - 10];

    let mut stmt = conn.prepare(&sql).unwrap();

    stmt.query_map([], |row| Ok(song(row)))
        .unwrap()
        .flatten()
        .collect()
}

pub fn collect_songs<P>(query: &str, params: P) -> Vec<Song>
where
    P: Params,
{
    let conn = conn();
    let mut stmt = conn.prepare(query).expect(query);

    stmt.query_map(params, |row| Ok(song(row)))
        .unwrap()
        .flatten()
        .collect()
}

pub fn song(row: &Row) -> Song {
    let path: String = row.get(3).unwrap();
    Song {
        name: row.get(0).unwrap(),
        disc: row.get(1).unwrap(),
        number: row.get(2).unwrap(),
        path: PathBuf::from(path),
        gain: row.get(4).unwrap(),
        album: row.get(5).unwrap(),
        artist: row.get(6).unwrap(),
        // folder: row.get(7).unwrap(),
        id: row.get(8).unwrap(),
    }
}

pub fn playback_device() -> String {
    let conn = conn();
    let mut stmt = conn.prepare("SELECT device FROM settings").unwrap();
    stmt.query_row([], |row| row.get(0)).unwrap()
}

pub fn set_playback_device(name: &str) {
    conn()
        .execute("UPDATE settings SET device = ? ", [name])
        .unwrap();
}

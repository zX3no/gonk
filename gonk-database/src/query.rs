use std::path::PathBuf;

use crate::conn;
use gonk_player::Song;
use rusqlite::*;

pub fn total_songs() -> usize {
    let conn = conn();
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM song").unwrap();
    stmt.query_row([], |row| row.get(0)).unwrap()
}

pub fn songs() -> Vec<Song> {
    collect_songs("SELECT *, rowid FROM song", params![])
}

pub fn artists() -> Vec<String> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT name FROM artist ORDER BY name COLLATE NOCASE")
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
        .prepare("SELECT name, artist_id FROM album ORDER BY artist_id COLLATE NOCASE")
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
        .prepare("SELECT name FROM album WHERE artist_id = ? ORDER BY name COLLATE NOCASE")
        .unwrap();

    stmt.query_map([artist], |row| row.get(0))
        .unwrap()
        .flatten()
        .collect()
}

pub fn songs_from_album(album: &str, artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT *, rowid FROM song WHERE artist_id = (?1) AND album_id = (?2) ORDER BY disc, number",
        params![artist, album],
    )
}

pub fn songs_by_artist(artist: &str) -> Vec<Song> {
    collect_songs(
        "SELECT *, rowid FROM song WHERE artist_id = ? ORDER BY album_id, disc, number",
        params![artist],
    )
}

pub fn songs_from_ids(ids: &[usize]) -> Vec<Song> {
    let conn = conn();
    let mut stmt = conn
        .prepare("SELECT *, rowid FROM song WHERE rowid = ?")
        .unwrap();

    //TODO: Maybe batch this?
    ids.iter()
        .flat_map(|id| stmt.query_row([id], |row| Ok(song(row))))
        .collect()
}

fn collect_songs<P>(query: &str, params: P) -> Vec<Song>
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

fn song(row: &Row) -> Song {
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

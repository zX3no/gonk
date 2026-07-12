use mu_core::vdb::Database;
use mu_core::Song;

pub fn find_song(
    db: &Database,
    artists: &[String],
    artist: &str,
    album: &str,
    disc: u8,
    num: u8,
) -> Option<Song> {
    if !artists.iter().any(|a| a == artist) {
        return None;
    }
    for al in db.albums_by_artist(artist) {
        if al.title != album {
            continue;
        }
        for song in &al.songs {
            if song.disc_number == disc && song.track_number == num {
                return Some(song.clone());
            }
        }
    }
    None
}

pub fn album_songs(db: &Database, artists: &[String], artist: &str, album: &str) -> Vec<Song> {
    if !artists.iter().any(|a| a == artist) {
        return Vec::new();
    }
    db.albums_by_artist(artist)
        .into_iter()
        .find(|a| a.title == album)
        .map(|a| a.songs.clone())
        .unwrap_or_default()
}

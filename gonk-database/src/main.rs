#![allow(unused)]
use gonk_database::*;

fn main() {
    init();

    let song = RawSong::new("artist", "album", "title", "path", 1, 1, 0.0);

    // let artist = artist(&song.text);
    // let new_artist = artist_alt(&new_song.text);
    // dbg!(artist, new_artist);

    // let album = album_alt(&new_song.text);
    let (artist, album) = artist_and_album(&song.text);
    dbg!(artist, album);

    bench(|| {
        let query = artist_and_album(&song.text);
    });
}

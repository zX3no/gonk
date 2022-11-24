use crate::*;
use rayon::slice::ParallelSliceMut;
use std::str::from_utf8_unchecked;

pub const fn artist_len(text: &[u8]) -> u16 {
    u16::from_le_bytes([text[0], text[1]])
}

pub const fn album_len(text: &[u8], artist_len: usize) -> u16 {
    u16::from_le_bytes([text[2 + artist_len], text[2 + artist_len + 1]])
}

pub const fn title_len(text: &[u8], artist_len: usize, album_len: usize) -> u16 {
    u16::from_le_bytes([
        text[2 + artist_len + 2 + album_len],
        text[2 + artist_len + 2 + album_len + 1],
    ])
}

pub const fn path_len(text: &[u8], artist_len: usize, album_len: usize, title_len: usize) -> u16 {
    u16::from_le_bytes([
        text[2 + artist_len + 2 + album_len + 2 + title_len],
        text[2 + artist_len + 2 + album_len + 2 + title_len + 1],
    ])
}

pub const fn artist(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;

    unsafe {
        let slice = text.get_unchecked(2..artist_len + 2);
        from_utf8_unchecked(slice)
    }
}

pub const fn album(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;

    unsafe {
        let slice = text.get_unchecked(2 + artist_len + 2..artist_len + 2 + album_len + 2);
        from_utf8_unchecked(slice)
    }
}

pub const fn title(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    let title_len = title_len(text, artist_len, album_len) as usize;

    unsafe {
        let slice = text.get_unchecked(
            2 + artist_len + 2 + album_len + 2..artist_len + 2 + album_len + 2 + title_len + 2,
        );
        from_utf8_unchecked(slice)
    }
}

pub const fn path(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    let title_len = title_len(text, artist_len, album_len) as usize;
    let path_len = path_len(text, artist_len, album_len, title_len) as usize;
    unsafe {
        let slice = text.get_unchecked(
            2 + artist_len + 2 + album_len + 2 + title_len + 2
                ..artist_len + 2 + album_len + 2 + title_len + 2 + path_len + 2,
        );
        from_utf8_unchecked(slice)
    }
}

pub const fn artist_and_album(text: &[u8]) -> (&str, &str) {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    unsafe {
        let artist = text.get_unchecked(2..artist_len + 2);
        let album = text.get_unchecked(2 + artist_len + 2..artist_len + 2 + album_len + 2);
        (from_utf8_unchecked(artist), from_utf8_unchecked(album))
    }
}

pub fn get(index: usize) -> Option<Song> {
    if let Some(mmap) = mmap() {
        let start = SONG_LEN * index;
        let bytes = mmap.get(start..start + SONG_LEN)?;
        Some(Song::from(bytes, index))
    } else {
        None
    }
}

pub fn ids(ids: &[usize]) -> Vec<Song> {
    if let Some(mmap) = mmap() {
        let mut songs = Vec::new();
        for id in ids {
            let start = SONG_LEN * id;
            let bytes = &mmap[start..start + SONG_LEN];
            songs.push(Song::from(bytes, *id));
        }
        songs
    } else {
        Vec::new()
    }
}

pub fn songs_from_album(ar: &str, al: &str) -> Vec<Song> {
    if let Some(mmap) = mmap() {
        let mut songs = Vec::new();
        let mut i = 0;
        while let Some(text) = mmap.get(i..i + TEXT_LEN) {
            let (artist, album) = artist_and_album(text);
            if artist == ar && album == al {
                songs.push(Song::from(&mmap[i..i + SONG_LEN], i / SONG_LEN));
            }
            i += SONG_LEN;
        }
        songs.sort_unstable();
        songs
    } else {
        Vec::new()
    }
}

pub fn albums_by_artist(ar: &str) -> Vec<String> {
    if let Some(mmap) = mmap() {
        let mut albums = Vec::new();
        let mut i = 0;
        while let Some(text) = mmap.get(i..i + TEXT_LEN) {
            let artist = artist(text);
            if artist == ar {
                albums.push(album(text).to_string());
            }
            i += SONG_LEN;
        }
        albums.sort_unstable_by_key(|album| album.to_ascii_lowercase());
        albums.dedup();
        albums
    } else {
        Vec::new()
    }
}

pub fn songs_by_artist(ar: &str) -> Vec<Song> {
    if let Some(mmap) = mmap() {
        let mut songs = Vec::new();
        let mut i = 0;
        while let Some(text) = mmap.get(i..i + TEXT_LEN) {
            let artist = artist(text);
            if artist == ar {
                let song_bytes = &mmap[i..i + SONG_LEN];
                songs.push(Song::from(song_bytes, i / SONG_LEN));
            }
            i += SONG_LEN;
        }
        songs.sort_unstable();
        songs
    } else {
        Vec::new()
    }
}

pub fn artists() -> Vec<String> {
    if let Some(mmap) = mmap() {
        let mut artists = Vec::new();
        let mut i = 0;
        while let Some(text) = mmap.get(i..i + TEXT_LEN) {
            artists.push(artist(text).to_string());
            i += SONG_LEN;
        }
        artists.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
        artists.dedup();
        artists
    } else {
        Vec::new()
    }
}

pub fn artists_albums_and_songs() -> (Vec<String>, Vec<(String, String)>, Vec<Song>) {
    if let Some(mmap) = mmap() {
        let songs: Vec<Song> = (0..len())
            .into_par_iter()
            .map(|i| {
                let pos = i * SONG_LEN;
                let bytes = &mmap[pos..pos + SONG_LEN];
                Song::from(bytes, i)
            })
            .collect();

        let mut albums: Vec<(&str, &str)> = songs
            .iter()
            .map(|song| (song.artist.as_str(), song.album.as_str()))
            .collect();
        albums.par_sort_unstable_by_key(|(artist, _album)| artist.to_ascii_lowercase());
        albums.dedup();
        let albums: Vec<(String, String)> = albums
            .into_iter()
            .map(|(artist, album)| (artist.to_owned(), album.to_owned()))
            .collect();

        let mut artists: Vec<String> = albums
            .iter()
            .map(|(artist, _album)| artist.clone())
            .collect();
        artists.dedup();

        (artists, albums, songs)
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    }
}

pub fn len() -> usize {
    if let Some(mmap) = mmap() {
        mmap.len() / SONG_LEN
    } else {
        0
    }
}

use crate::*;
use std::str::from_utf8_unchecked;

pub fn artist(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    unsafe {
        let end = text.iter().position(|&c| c == b'\0').unwrap();
        from_utf8_unchecked(&text[..end])
    }
}

pub fn album(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut start = 0;
    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            if start == 0 {
                start = i + 1;
            } else {
                return unsafe { from_utf8_unchecked(&text[start..i]) };
            }
        }
    }
    unreachable!();
}

pub fn title(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut found = 0;
    let mut start = 0;
    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            found += 1;
            if found == 2 {
                start = i + 1;
            } else if found == 3 {
                return unsafe { from_utf8_unchecked(&text[start..i]) };
            }
        }
    }
    unreachable!();
}

pub fn path(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut found = 0;
    let mut start = 0;
    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            found += 1;
            if found == 3 {
                start = i;
            } else if found == 4 {
                return unsafe { from_utf8_unchecked(&text[start + 1..i]) };
            }
        }
    }
    unreachable!();
}

pub fn artist_and_album(text: &[u8]) -> (&str, &str) {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut found = 0;
    let mut end_artist = 0;
    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            found += 1;
            if found == 1 {
                end_artist = i;
            } else if found == 2 {
                return unsafe {
                    (
                        from_utf8_unchecked(&text[..end_artist]),
                        from_utf8_unchecked(&text[end_artist + 1..i]),
                    )
                };
            }
        }
    }
    unreachable!();
}

pub fn get(index: usize) -> Option<Song> {
    optick::event!();
    if let Some(mmap) = mmap() {
        let start = SONG_LEN * index;
        let bytes = mmap.get(start..start + SONG_LEN)?;
        Some(Song::from(bytes, index))
    } else {
        None
    }
}

pub fn ids(ids: &[usize]) -> Vec<Song> {
    optick::event!();
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
    optick::event!();
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
        songs
    } else {
        Vec::new()
    }
}

pub fn albums_by_artist(ar: &str) -> Vec<String> {
    optick::event!();
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
    optick::event!();
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
        songs
    } else {
        Vec::new()
    }
}

// pub fn songs() -> Vec<Song> {
//     optick::event!();
//         if let Some(mmap) = mmap() {
//     let mut songs = Vec::new();
//     let mut i = 0;
//     while let Some(bytes) = mmap.get(i..i + SONG_LEN) {
//         songs.push(Song::from(bytes, i / SONG_LEN));
//         i += SONG_LEN;
//     }
//     songs
// }

pub fn par_songs() -> Vec<Song> {
    optick::event!();
    if let Some(mmap) = mmap() {
        (0..len())
            .into_par_iter()
            .map(|i| {
                let pos = i * SONG_LEN;
                let bytes = &mmap[pos..pos + SONG_LEN];
                Song::from(bytes, i)
            })
            .collect()
    } else {
        Vec::new()
    }
}

///(Artist, Album)
pub fn albums() -> Vec<(String, String)> {
    optick::event!();
    if let Some(mmap) = mmap() {
        let mut albums = Vec::new();
        let mut i = 0;
        while let Some(text) = mmap.get(i..i + TEXT_LEN) {
            albums.push((artist(text).to_string(), album(text).to_string()));
            i += SONG_LEN;
        }
        albums.sort_unstable_by_key(|(_, artist)| artist.to_ascii_lowercase());
        albums.dedup();
        albums
    } else {
        Vec::new()
    }
}

pub fn artists() -> Vec<String> {
    optick::event!();
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

pub fn len() -> usize {
    if let Some(mmap) = mmap() {
        mmap.len() / SONG_LEN
    } else {
        0
    }
}

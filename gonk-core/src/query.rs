use crate::*;
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
        let slice = text.get_unchecked(2 + artist_len + 2..2 + artist_len + 2 + album_len);
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
            2 + artist_len + 2 + album_len + 2..2 + artist_len + 2 + album_len + 2 + title_len,
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
                ..2 + artist_len + 2 + album_len + 2 + title_len + 2 + path_len,
        );
        from_utf8_unchecked(slice)
    }
}

pub const fn artist_and_album(text: &[u8]) -> (&str, &str) {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    unsafe {
        let artist = text.get_unchecked(2..2 + artist_len);
        let album = text.get_unchecked(2 + artist_len + 2..2 + artist_len + 2 + album_len);
        (from_utf8_unchecked(artist), from_utf8_unchecked(album))
    }
}

pub fn len() -> usize {
    if let Some(mmap) = mmap() {
        mmap.len() / SONG_LEN
    } else {
        0
    }
}

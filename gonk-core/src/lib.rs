#![feature(extend_one, test, const_float_bits_conv, const_slice_index)]
#![allow(clippy::missing_safety_doc)]
use std::{env, fmt::Debug, fs, ops::Range, path::PathBuf, str::from_utf8_unchecked};

mod flac_decoder;
mod index;
mod playlist;

pub mod db;
pub mod log;
pub mod profiler;
pub mod settings;

pub use db::*;
pub use index::*;
pub use playlist::*;
pub use settings::*;

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

#[derive(Debug)]
pub struct Artist {
    pub albums: Vec<Album>,
}

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Song {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub disc_number: u8,
    pub track_number: u8,
    pub path: String,
    pub gain: f32,
}

pub fn gonk_path() -> PathBuf {
    let gonk = if cfg!(windows) {
        PathBuf::from(&env::var("APPDATA").unwrap())
    } else {
        PathBuf::from(&env::var("HOME").unwrap()).join(".config")
    }
    .join("gonk");

    if !gonk.exists() {
        fs::create_dir_all(&gonk).unwrap();
    }

    gonk
}

pub fn settings_path() -> PathBuf {
    let mut path = database_path();
    path.pop();
    path.push("settings.db");
    path
}

pub fn database_path() -> PathBuf {
    let gonk = gonk_path();

    //Backwards compatibility for older versions of gonk
    let old_db = gonk.join("gonk_new.db");
    let db = gonk.join("gonk.db");

    if old_db.exists() {
        fs::rename(old_db, &db).unwrap();
    }

    db
}

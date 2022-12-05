#![feature(extend_one, test, const_float_bits_conv, const_slice_index)]
#![allow(clippy::missing_safety_doc)]
use std::{
    env, error::Error, fmt::Debug, fs, mem::size_of, ops::Range, path::PathBuf, str::from_utf8,
};

mod flac_decoder;
mod index;
mod playlist;
mod settings;

pub mod db;
pub mod log;
pub mod profiler;
pub mod raw_song;

pub use db::*;
pub use index::*;
pub use playlist::*;
pub use raw_song::*;

#[derive(Debug)]
pub struct Artist {
    pub albums: Vec<Album>,
}

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub disc_number: u8,
    pub track_number: u8,
    pub path: String,
    pub gain: f32,
}

//TODO: Need to write a validate entire database function
//should help catch problems like this.
impl From<&[u8]> for Song {
    fn from(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() == SONG_LEN);
        let text = bytes.get(..TEXT_LEN).unwrap();
        let artist = artist(text);
        let album = album(text);
        let title = title(text);
        let path = path(text);

        let track_number = bytes[NUMBER_POS];
        let disc_number = bytes[DISC_POS];

        let gain = f32::from_le_bytes([
            bytes[SONG_LEN - 4],
            bytes[SONG_LEN - 3],
            bytes[SONG_LEN - 2],
            bytes[SONG_LEN - 1],
        ]);

        Self {
            artist: artist.to_string(),
            album: album.to_string(),
            title: title.to_string(),
            path: path.to_string(),
            track_number,
            disc_number,
            gain,
        }
    }
}

impl From<&RawSong> for Song {
    fn from(raw: &RawSong) -> Self {
        Song {
            title: raw.title().to_string(),
            album: raw.album().to_string(),
            artist: raw.artist().to_string(),
            disc_number: raw.disc,
            track_number: raw.number,
            path: raw.path().to_string(),
            gain: raw.gain,
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::{raw_song::RawSong, settings::Settings, *};
    extern crate test;

    #[test]
    fn validate_all() {
        for albums in Database::raw().values() {
            for album in albums {
                for s in &album.songs {
                    let raw = RawSong::from(s);
                    raw.validate().unwrap();
                }
            }
        }
    }

    #[test]
    fn clamp_song() {
        let song = RawSong::new(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1,
            1,
            0.25,
        );
        assert_eq!(song.artist().len(), 126);
        // assert_eq!(song.album().len(), 127);
        assert_eq!(song.title().len(), 127);
        assert_eq!(song.path().len(), 134);
        assert_eq!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".len(), 134);
    }

    #[test]
    fn settings() {
        let mut settings = unsafe { Settings::new() };
        let song = RawSong::new("artist", "album", "title", "path", 1, 1, 0.25);
        settings.queue.push(song);

        let bytes = settings.as_bytes();
        let new_settings = unsafe { Settings::from_bytes(&bytes, settings.file) };

        assert_eq!(settings.volume, new_settings.volume);
        assert_eq!(settings.index, new_settings.index);
        assert_eq!(settings.elapsed, new_settings.elapsed);
        assert_eq!(settings.output_device, new_settings.output_device);
        assert_eq!(settings.music_folder, new_settings.music_folder);
    }

    #[test]
    fn database() {
        let mut db = Vec::new();
        for i in 0..10_000 {
            let song = RawSong::new(
                &format!("{i} artist"),
                &format!("{i} album"),
                &format!("{i} title"),
                &format!("{i} path"),
                1,
                1,
                0.25,
            );
            db.extend(song.as_bytes());
        }

        assert_eq!(db.len(), 5280000);
        assert_eq!(db.len() / SONG_LEN, 10_000);
        assert_eq!(artist(&db[..TEXT_LEN]), "0 artist");
        assert_eq!(album(&db[..TEXT_LEN]), "0 album");
        assert_eq!(title(&db[..TEXT_LEN]), "0 title");
        assert_eq!(path(&db[..TEXT_LEN]), "0 path");

        assert_eq!(
            artist(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 artist"
        );
        assert_eq!(
            album(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 album"
        );
        assert_eq!(
            title(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 title"
        );
        assert_eq!(
            path(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 path"
        );

        let song = Song::from(&db[..SONG_LEN]);
        assert_eq!(song.artist, "0 artist");
        assert_eq!(song.album, "0 album");
        assert_eq!(song.title, "0 title");
        assert_eq!(song.path, "0 path");
        assert_eq!(song.track_number, 1);
        assert_eq!(song.disc_number, 1);
        assert_eq!(song.gain, 0.25);

        let song = Song::from(&db[SONG_LEN * 9999..SONG_LEN * 10000]);
        assert_eq!(song.artist, "9999 artist");
        assert_eq!(song.album, "9999 album");
        assert_eq!(song.title, "9999 title");
        assert_eq!(song.path, "9999 path");
        assert_eq!(song.track_number, 1);
        assert_eq!(song.disc_number, 1);
        assert_eq!(song.gain, 0.25);
    }
}

//! The physical database is a file on disk that stores song information.
//! This information includes the artist, album, title, disc number, track number, path and replay gain.
//!
//! The virtual database stores key value pairs.
//! It is used for quering artists, albums and songs.
//!
//! `Index` is a wrapper over a `Vec<T>` plus an index. Kind of like a circular buffer but the data is usually constant.
//! It's useful for moving up and down the selection of a UI element.
use std::{
    env,
    error::Error,
    fs::{self},
    path::{Path, PathBuf},
};

pub use crate::{
    db::{Album, Artist, Song},
    // old_db::{Album, Artist, Song},
    playlist::Playlist,
};
pub use flac_decoder::*;
pub use index::*;

pub mod db;
pub mod flac_decoder;
pub mod index;
pub mod log;
pub mod playlist;
pub mod profiler;
pub mod settings;
pub mod strsim;
pub mod vdb;

///Escape potentially problematic strings.
pub fn escape(input: &str) -> String {
    input.replace('\n', "").replace('\t', "    ")
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
    let mut path = gonk_path();
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

trait Serialize {
    fn serialize(&self) -> String;
}

trait Deserialize
where
    Self: Sized,
{
    type Error;

    fn deserialize(s: &str) -> Result<Self, Self::Error>;
}

#[cfg(test)]
mod tests {

    #[test]
    fn no_alloc() {
        use super::*;
        use rayon::prelude::*;
        use std::fs::File;
        use std::io::BufWriter;
        use std::io::Write;
        use std::time::Instant;
        let paths: Vec<walkdir::DirEntry> = walkdir::WalkDir::new("D:\\OneDrive\\Music")
            .into_iter()
            .flatten()
            .filter(|path| match path.path().extension() {
                Some(ex) => {
                    matches!(ex.to_str(), Some("flac" | "mp3" | "ogg"))
                }
                None => false,
            })
            .collect();

        let songs: Vec<Song> = paths
            .into_par_iter()
            .flat_map(|dir| Song::try_from(dir.path()))
            .collect();

        dbg!(songs.capacity(), songs.len());

        let now = Instant::now();
        let file = File::create("test.db").unwrap();
        let mut writer = BufWriter::new(file);

        for song in songs {
            writer.write_all(&escape(&song.title).into_bytes()).unwrap();
            writer.write_all(&[b'\t']).unwrap();

            writer.write_all(&escape(&song.album).into_bytes()).unwrap();
            writer.write_all(&[b'\t']).unwrap();

            writer
                .write_all(&escape(&song.artist).into_bytes())
                .unwrap();
            writer.write_all(&[b'\t']).unwrap();

            writer
                .write_all(&song.disc_number.to_string().into_bytes())
                .unwrap();
            writer.write_all(&[b'\t']).unwrap();

            writer
                .write_all(&song.track_number.to_string().into_bytes())
                .unwrap();
            writer.write_all(&[b'\t']).unwrap();

            if song.gain == 0.0 {
                writer.write_all(b"0.0").unwrap();
            } else {
                writer
                    .write_all(&song.gain.to_string().into_bytes())
                    .unwrap();
            }

            writer.write_all(&[b'\n']).unwrap();
        }

        writer.flush().unwrap();
        dbg!(now.elapsed());
    }

    #[test]
    fn alloc() {
        use super::*;
        use rayon::prelude::*;
        use std::time::Instant;
        let paths: Vec<walkdir::DirEntry> = walkdir::WalkDir::new("D:\\OneDrive\\Music")
            .into_iter()
            .flatten()
            .filter(|path| match path.path().extension() {
                Some(ex) => {
                    matches!(ex.to_str(), Some("flac" | "mp3" | "ogg"))
                }
                None => false,
            })
            .collect();

        let songs: Vec<Song> = paths
            .into_par_iter()
            .flat_map(|dir| Song::try_from(dir.path()))
            .collect();

        dbg!(songs.capacity(), songs.len());

        let now = Instant::now();
        let mut buffer: Vec<u8> = Vec::new();

        for song in songs {
            buffer.extend(escape(&song.title).into_bytes());
            buffer.push(b'\t');

            buffer.extend(escape(&song.album).into_bytes());
            buffer.push(b'\t');

            buffer.extend(escape(&song.artist).into_bytes());
            buffer.push(b'\t');

            buffer.extend(song.disc_number.to_string().into_bytes());
            buffer.push(b'\t');

            buffer.extend(song.track_number.to_string().into_bytes());
            buffer.push(b'\t');

            let gain = if song.gain == 0.0 {
                String::from("0.0")
            } else {
                song.gain.to_string()
            };
            buffer.extend(gain.into_bytes());

            buffer.push(b'\n');
        }

        fs::write("test.db", buffer).unwrap();
        dbg!(now.elapsed());
        // dbg!(buffer.capacity(), buffer.len());
    }
}

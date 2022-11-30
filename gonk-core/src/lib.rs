#![feature(test)]
#![feature(const_slice_index)]
#![feature(const_float_bits_conv)]
#![allow(clippy::missing_safety_doc)]
use db::*;
use raw_song::*;
use std::{
    env, error::Error, fmt::Debug, fs, mem::size_of, ops::Range, path::PathBuf, str::from_utf8,
};

mod flac_decoder;
mod index;
mod playlist;
mod raw_song;
mod settings;

pub mod db;
pub mod log;
pub mod profiler;

pub use db::*;
pub use index::*;
pub use playlist::*;

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
    use std::io::{BufWriter, Write};

    use crate::{raw_song::RawSong, settings::Settings, *};
    use rayon::prelude::{IntoParallelIterator, ParallelIterator, ParallelSliceMut};
    extern crate test;
    use tempfile::tempfile;
    use test::Bencher;

    #[bench]
    fn bench_collect_all(b: &mut Bencher) {
        let file = tempfile().unwrap();

        let mut writer = BufWriter::new(&file);
        for i in 0..10_000 {
            let song = RawSong::new(
                &format!("{} artist", i),
                &format!("{} album", i),
                &format!("{} title", i),
                &format!("{} path", i),
                1,
                1,
                0.25,
            );
            writer.write_all(&song.as_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };

        b.iter(|| {
            let songs: Vec<Song> = (0..mmap.len() / SONG_LEN)
                .into_par_iter()
                .map(|i| {
                    let pos = i * SONG_LEN;
                    let bytes = &mmap[pos..pos + SONG_LEN];
                    Song::from(bytes)
                })
                .collect();
            assert_eq!(songs.len(), 10_000);

            let mut albums: Vec<(&str, &str)> = songs
                .iter()
                .map(|song| (song.artist.as_str(), song.album.as_str()))
                .collect();
            albums.par_sort_unstable_by_key(|(artist, _album)| artist.to_ascii_lowercase());
            albums.dedup();
            assert_eq!(albums.len(), 10_000);

            let albums: Vec<(String, String)> = albums
                .into_iter()
                .map(|(artist, album)| (artist.to_owned(), album.to_owned()))
                .collect();
            assert_eq!(albums.len(), 10_000);

            let mut artists: Vec<String> = albums
                .iter()
                .map(|(artist, _album)| artist.clone())
                .collect();
            artists.dedup();

            assert_eq!(artists.len(), 10_000);
        });
    }
    #[bench]
    fn bench_collect_artist_single(b: &mut Bencher) {
        let file = tempfile().unwrap();

        let mut writer = BufWriter::new(&file);
        for i in 0..10_000 {
            let song = RawSong::new(
                &format!("{} artist", i),
                &format!("{} album", i),
                &format!("{} title", i),
                &format!("{} path", i),
                1,
                1,
                0.25,
            );
            writer.write_all(&song.as_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };

        b.iter(|| {
            let mut songs = Vec::new();
            let mut i = 0;
            while let Some(text) = mmap.get(i..i + TEXT_LEN) {
                let artist = artist(text);
                if artist == "9999 artist" {
                    let song_bytes = &mmap[i..i + SONG_LEN];
                    songs.push(Song::from(song_bytes));
                }
                i += SONG_LEN;
            }
            assert_eq!(songs.len(), 1);
        });
    }
    #[bench]
    fn bench_collect_artist(b: &mut Bencher) {
        let file = tempfile().unwrap();

        let mut writer = BufWriter::new(&file);
        let song = RawSong::new("artist", "album", "title", "path", 1, 1, 0.25);
        for _ in 0..10_000 {
            writer.write_all(&song.as_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };

        b.iter(|| {
            let mut songs = Vec::new();
            let mut i = 0;
            while let Some(text) = mmap.get(i..i + TEXT_LEN) {
                let artist = artist(text);
                if artist == "artist" {
                    let song_bytes = &mmap[i..i + SONG_LEN];
                    songs.push(Song::from(song_bytes));
                }
                i += SONG_LEN;
            }
            assert_eq!(songs.len(), 10000);
        });
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
        assert_eq!(song.album().len(), 127);
        assert_eq!(song.title().len(), 127);
        assert_eq!(song.path().len(), 134);
        assert_eq!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".len(), 134);
    }

    #[test]
    fn settings() {
        let mut settings = Settings::default();
        let song = RawSong::new("artist", "album", "title", "path", 1, 1, 0.25);
        settings.queue.push(song);

        let bytes = settings.as_bytes();
        let new_settings = Settings::from(bytes).unwrap();

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
                &format!("{} artist", i),
                &format!("{} album", i),
                &format!("{} title", i),
                &format!("{} path", i),
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

//! The physical(on disk) database
//!
//! Scanning and parsing music files and writing those tags to a database.
//!
//! TODO: Describe the database format.
//!
use crate::*;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    thread::{self, JoinHandle},
};
use walkdir::{DirEntry, WalkDir};

pub const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() + size_of::<u8>() + size_of::<f32>();
pub const TEXT_LEN: usize = 522;
pub const NUMBER_POS: usize = SONG_LEN - 1 - size_of::<f32>() - size_of::<u8>();
pub const DISC_POS: usize = SONG_LEN - 1 - size_of::<f32>();
pub const GAIN_POS: Range<usize> = SONG_LEN - size_of::<f32>()..SONG_LEN;

#[derive(Debug)]
pub enum ScanResult {
    Completed,
    CompletedWithErrors(Vec<String>),
    FileInUse,
}

pub fn create(path: impl ToString) -> JoinHandle<ScanResult> {
    let path = path.to_string();
    thread::spawn(|| {
        let mut db_path = database_path();
        db_path.pop();
        db_path.push("temp.db");

        match File::create(&db_path) {
            Ok(file) => {
                let mut writer = BufWriter::new(&file);

                let paths: Vec<DirEntry> = WalkDir::new(path)
                    .into_iter()
                    .flatten()
                    .filter(|path| match path.path().extension() {
                        Some(ex) => {
                            matches!(ex.to_str(), Some("flac" | "mp3" | "ogg"))
                        }
                        None => false,
                    })
                    .collect();

                let songs: Vec<_> = paths
                    .into_par_iter()
                    .map(|dir| path_to_bytes(dir.path()))
                    .collect();

                let errors: Vec<String> = songs
                    .iter()
                    .filter_map(|song| {
                        if let Err(err) = song {
                            Some(err.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                let songs: Vec<[u8; SONG_LEN]> = songs.into_iter().flatten().collect();

                for song in songs {
                    writer.write_all(&song).unwrap();
                }

                writer.flush().unwrap();

                //Remove old database and replace it with new.
                fs::rename(db_path, database_path()).unwrap();

                let _db = vdb::create().unwrap();

                if errors.is_empty() {
                    ScanResult::Completed
                } else {
                    ScanResult::CompletedWithErrors(errors)
                }
            }
            Err(_) => ScanResult::FileInUse,
        }
    })
}
pub fn read() -> Result<Vec<Song>, Box<dyn Error + Send + Sync>> {
    let bytes = fs::read(database_path())?;

    let song_count = bytes.len() / db::SONG_LEN;

    if bytes.len() % db::SONG_LEN != 0 {
        return Err("Size of database is incorrect")?;
    }

    let songs: Vec<Song> = (0..song_count)
        .into_par_iter()
        .flat_map(|i| {
            let pos = i * db::SONG_LEN;
            let bytes = &bytes.get(pos..pos + db::SONG_LEN).ok_or("Invalid song")?;
            db::bytes_to_song(bytes)
        })
        .collect();

    Ok(songs)
}

pub fn bytes_to_song(bytes: &[u8]) -> Result<Song, Box<dyn Error + Send + Sync>> {
    if bytes.len() != SONG_LEN {
        return Err("Slice size does not match song length")?;
    }

    //Find the positions of the data.
    let text = bytes.get(..TEXT_LEN).ok_or("Could not get text segment")?;

    let artist_len = u16::from_le_bytes([text[0], text[1]]) as usize;
    let slice = text
        .get(2 + artist_len..2 + artist_len + 2)
        .ok_or("Artist length is corrupted")?;
    let album_len = u16::from_le_bytes(slice.try_into()?) as usize;

    let slice = text
        .get(2 + artist_len + 2 + album_len..2 + artist_len + 2 + album_len + 2)
        .ok_or("Album length is corrupted")?;

    let title_len = u16::from_le_bytes(slice.try_into()?) as usize;

    let slice = text
        .get(
            2 + artist_len + 2 + album_len + 2 + title_len
                ..2 + artist_len + 2 + album_len + 2 + title_len + 2,
        )
        .ok_or("Title length is corrupted")?;

    let path_len = u16::from_le_bytes(slice.try_into()?) as usize;

    //Collect the data.

    let slice = text.get(2..artist_len + 2).ok_or("Invalid artist length")?;
    let artist = from_utf8(slice)?;

    let slice = text
        .get(2 + artist_len + 2..2 + artist_len + 2 + album_len)
        .ok_or("Invalid album length")?;
    let album = from_utf8(slice)?;

    let slice = text
        .get(2 + artist_len + 2 + album_len + 2..2 + artist_len + 2 + album_len + 2 + title_len)
        .ok_or("Invalid title length")?;
    let title = from_utf8(slice)?;
    let slice = text
        .get(
            2 + artist_len + 2 + album_len + 2 + title_len + 2
                ..2 + artist_len + 2 + album_len + 2 + title_len + 2 + path_len,
        )
        .ok_or("Invalid path length")?;
    let path = from_utf8(slice)?;

    if !(path.ends_with("flac") | path.ends_with("mp3") | path.ends_with("ogg")) {
        return Err("Path requires an audio file extension")?;
    }

    let track_number = bytes.get(NUMBER_POS).ok_or("Invalid track")?;
    let disc_number = bytes.get(DISC_POS).ok_or("Invalid disc")?;

    let gain = f32::from_le_bytes(bytes.get(GAIN_POS).ok_or("Invalid gain")?.try_into()?);

    Ok(Song {
        title: title.to_string(),
        album: album.to_string(),
        artist: artist.to_string(),
        disc_number: *disc_number,
        track_number: *track_number,
        path: path.to_string(),
        gain,
    })
}

pub fn song_to_bytes(
    artist: &str,
    album: &str,
    title: &str,
    path: &str,
    number: u8,
    disc: u8,
    gain: f32,
) -> [u8; SONG_LEN] {
    let mut song = [0; SONG_LEN];

    if path.len() > TEXT_LEN {
        //If there is invalid utf8 in the panic message, rust will panic.
        panic!("PATH IS TOO LONG! {:?}", OsString::from(path));
    }

    let mut artist = artist.to_string();
    let mut album = album.to_string();
    let mut title = title.to_string();

    //Forcefully fit the artist, album, title and path into 522 bytes.
    //There are 4 u16s included in the text so those are subtracted too.
    let mut i = 0;
    while artist.len() + album.len() + title.len() + path.len() > TEXT_LEN - (4 * size_of::<u16>())
    {
        if i % 3 == 0 {
            artist.pop();
        } else if i % 3 == 1 {
            album.pop();
        } else {
            title.pop();
        }
        i += 1;
    }

    if i != 0 {
        // log!(
        //     "Warning: {} overflowed {} bytes! Metadata will be truncated.",
        //     path,
        //     SONG_LEN
        // );
    }

    let artist_len = (artist.len() as u16).to_le_bytes();
    song[0..2].copy_from_slice(&artist_len);
    song[2..2 + artist.len()].copy_from_slice(artist.as_bytes());

    let album_len = (album.len() as u16).to_le_bytes();
    song[2 + artist.len()..2 + artist.len() + 2].copy_from_slice(&album_len);
    song[2 + artist.len() + 2..2 + artist.len() + 2 + album.len()]
        .copy_from_slice(album.as_bytes());

    let title_len = (title.len() as u16).to_le_bytes();
    song[2 + artist.len() + 2 + album.len()..2 + artist.len() + 2 + album.len() + 2]
        .copy_from_slice(&title_len);
    song[2 + artist.len() + 2 + album.len() + 2
        ..2 + artist.len() + 2 + album.len() + 2 + title.len()]
        .copy_from_slice(title.as_bytes());

    let path_len = (path.len() as u16).to_le_bytes();
    song[2 + artist.len() + 2 + album.len() + 2 + title.len()
        ..2 + artist.len() + 2 + album.len() + 2 + title.len() + 2]
        .copy_from_slice(&path_len);
    song[2 + artist.len() + 2 + album.len() + 2 + title.len() + 2
        ..2 + artist.len() + 2 + album.len() + 2 + title.len() + 2 + path.len()]
        .copy_from_slice(path.as_bytes());

    song[NUMBER_POS] = number;
    song[DISC_POS] = disc;
    song[GAIN_POS].copy_from_slice(&gain.to_le_bytes());

    song
}

pub fn path_to_bytes(path: &'_ Path) -> Result<[u8; SONG_LEN], String> {
    // let _ex = path.extension().unwrap();
    match read_metadata(path) {
        Ok(metadata) => {
            let number = metadata
                .get("TRACKNUMBER")
                .unwrap_or(&String::from("1"))
                .parse()
                .unwrap_or(1);

            let disc = metadata
                .get("DISCNUMBER")
                .unwrap_or(&String::from("1"))
                .parse()
                .unwrap_or(1);

            let mut gain = 0.0;
            if let Some(db) = metadata.get("REPLAYGAIN_TRACK_GAIN") {
                let g = db.replace(" dB", "");
                if let Ok(db) = g.parse::<f32>() {
                    gain = 10.0f32.powf(db / 20.0);
                }
            }

            let artist = match metadata.get("ALBUMARTIST") {
                Some(artist) => artist.as_str(),
                None => match metadata.get("ARTIST") {
                    Some(artist) => artist.as_str(),
                    None => "Unknown Artist",
                },
            };

            let album = match metadata.get("ALBUM") {
                Some(album) => album.as_str(),
                None => "Unknown Album",
            };

            let title = match metadata.get("TITLE") {
                Some(title) => title.as_str(),
                None => "Unknown Title",
            };

            Ok(song_to_bytes(
                artist,
                album,
                title,
                path.to_str().unwrap(),
                number,
                disc,
                gain,
            ))
        }
        Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
    }
}

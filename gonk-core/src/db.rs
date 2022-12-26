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

static mut LEN: usize = 0;

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

impl Song {
    pub fn to_bytes(&self) -> [u8; SONG_LEN] {
        profile!();
        let mut song = [0; SONG_LEN];

        if self.path.len() > TEXT_LEN {
            //If there is invalid utf8 in the panic message, rust will panic.
            panic!("PATH IS TOO LONG! {:?}", OsString::from(&self.path));
        }

        let mut artist = self.artist.to_string();
        let mut album = self.album.to_string();
        let mut title = self.title.to_string();
        let path = self.path.to_string();

        //Forcefully fit the artist, album, title and path into 522 bytes.
        //There are 4 u16s included in the text so those are subtracted too.
        let mut i = 0;
        while artist.len() + album.len() + title.len() + path.len()
            > TEXT_LEN - (4 * size_of::<u16>())
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
            log!(
                "Warning: {} overflowed {} bytes! Metadata will be truncated.",
                path,
                SONG_LEN
            );
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

        song[NUMBER_POS] = self.track_number;
        song[DISC_POS] = self.disc_number;
        song[GAIN_POS].copy_from_slice(&self.gain.to_le_bytes());

        song
    }
}

#[derive(Debug, Default)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

#[derive(Debug, Default)]
pub struct Artist {
    pub albums: Vec<Album>,
}

#[derive(Debug)]
pub enum ScanResult {
    Completed,
    CompletedWithErrors(Vec<String>),
    FileInUse,
}

pub fn len() -> usize {
    unsafe { LEN }
}

pub fn reset() -> Result<(), Box<dyn Error>> {
    fs::remove_file(settings_path())?;
    if database_path().exists() {
        fs::remove_file(database_path())?;
    }
    Ok(())
}

pub fn create(path: impl ToString) -> JoinHandle<ScanResult> {
    let path = path.to_string();
    thread::spawn(move || {
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
    let bytes = match fs::read(database_path()) {
        Ok(bytes) => bytes,
        Err(error) => {
            return match error.kind() {
                std::io::ErrorKind::NotFound => Ok(Vec::new()),
                _ => Err(error)?,
            }
        }
    };

    //This may not represent the amount of songs in the database.
    let song_count = bytes.len() / db::SONG_LEN;
    //Keep track of the database size.
    unsafe { LEN = song_count };

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
    profile!();
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

pub fn path_to_bytes(path: &'_ Path) -> Result<[u8; SONG_LEN], String> {
    profile!();
    let extension = path.extension().ok_or("Path is not audio")?;

    if extension != "flac" {
        use symphonia::{
            core::{formats::FormatOptions, io::*, meta::*, probe::Hint},
            default::get_probe,
        };

        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
        };

        let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

        let mut probe = match get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions {
                limit_visual_bytes: Limit::Maximum(1),
                ..Default::default()
            },
        ) {
            Ok(probe) => probe,
            Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
        };

        let mut title = String::from("Unknown Title");
        let mut album = String::from("Unknown Album");
        let mut artist = String::from("Unknown Artist");
        let mut track_number = 1;
        let mut disc_number = 1;
        let mut gain = 0.0;

        let mut metadata_revision = probe.format.metadata();
        let mut metadata = probe.metadata.get();

        //TODO: WTF IS THIS???
        let metadata = match metadata_revision.skip_to_latest() {
            Some(metadata) => metadata,
            None => match &mut metadata {
                Some(metadata) => match metadata.skip_to_latest() {
                    Some(metadata) => metadata,
                    None => {
                        let song = Song {
                            title,
                            album,
                            artist,
                            disc_number,
                            track_number,
                            path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                            gain,
                        };
                        return Ok(song.to_bytes());
                    }
                },
                None => {
                    let song = Song {
                        title,
                        album,
                        artist,
                        disc_number,
                        track_number,
                        path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                        gain,
                    };
                    return Ok(song.to_bytes());
                }
            },
        };

        for tag in metadata.tags() {
            if let Some(std_key) = tag.std_key {
                match std_key {
                    StandardTagKey::AlbumArtist => artist = tag.value.to_string(),
                    StandardTagKey::Artist if artist == "Unknown Artist" => {
                        artist = tag.value.to_string()
                    }
                    StandardTagKey::Album => album = tag.value.to_string(),
                    StandardTagKey::TrackTitle => title = tag.value.to_string(),
                    StandardTagKey::TrackNumber => {
                        let num = tag.value.to_string();
                        if let Some((num, _)) = num.split_once('/') {
                            track_number = num.parse().unwrap_or(1);
                        } else {
                            track_number = num.parse().unwrap_or(1);
                        }
                    }
                    StandardTagKey::DiscNumber => {
                        let num = tag.value.to_string();
                        if let Some((num, _)) = num.split_once('/') {
                            disc_number = num.parse().unwrap_or(1);
                        } else {
                            disc_number = num.parse().unwrap_or(1);
                        }
                    }
                    StandardTagKey::ReplayGainTrackGain => {
                        let tag = tag.value.to_string();
                        let (_, value) = tag.split_once(' ').ok_or("Invalid replay gain.")?;
                        let db = value.parse().unwrap_or(0.0);
                        gain = 10.0f32.powf(db / 20.0);
                    }
                    _ => (),
                }
            }
        }

        let song = Song {
            title,
            album,
            artist,
            disc_number,
            track_number,
            path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
            gain,
        };
        Ok(song.to_bytes())
    } else {
        match read_metadata(path) {
            Ok(metadata) => {
                let track_number = metadata
                    .get("TRACKNUMBER")
                    .unwrap_or(&String::from("1"))
                    .parse()
                    .unwrap_or(1);

                let disc_number = metadata
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

                let song = Song {
                    title: title.to_string(),
                    album: album.to_string(),
                    artist: artist.to_string(),
                    disc_number,
                    track_number,
                    path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                    gain,
                };
                Ok(song.to_bytes())
            }
            Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
        }
    }
}

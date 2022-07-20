#![allow(unused)]
use std::{
    fmt::{Debug, Display},
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
    num,
    ops::Range,
    path::Path,
    str::{from_utf8, from_utf8_unchecked},
    time::Instant,
};
use symphonia::{
    core::{
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::{MetadataOptions, MetadataRevision, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

use memmap2::Mmap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use walkdir::DirEntry;

const TEXT_LEN: usize = 510;
const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() * 2;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone, Debug)]
struct Song {
    //Name, album, artist, path are all crammed into this space.
    text: [u8; TEXT_LEN],
    number: u8,
    disc: u8,
}

impl Song {
    pub fn new(name: &str, album: &str, artist: &str, path: &str, number: u8, disc: u8) -> Self {
        let len = name.len() + album.len() + artist.len() + path.len();
        if len > TEXT_LEN {
            panic!("Text is '{}' bytes to many!", len - TEXT_LEN);
        } else {
            let name = [name.as_bytes(), &[b'\0']].concat();
            let album = [album.as_bytes(), &[b'\0']].concat();
            let artist = [artist.as_bytes(), &[b'\0']].concat();
            let path = [path.as_bytes(), &[b'\0']].concat();

            let mut text = [0u8; TEXT_LEN];
            let name_pos = name.len();
            let album_pos = name_pos + album.len();
            let artist_pos = album_pos + artist.len();
            let path_pos = artist_pos + path.len();

            text[..name_pos].copy_from_slice(&name);
            text[name_pos..album_pos].copy_from_slice(&album);
            text[album_pos..artist_pos].copy_from_slice(&artist);
            text[artist_pos..path_pos].copy_from_slice(&path);

            Self { text, number, disc }
        }
    }
    pub fn into_bytes(mut self) -> [u8; SONG_LEN] {
        let mut song = [0u8; SONG_LEN];
        song[0..TEXT_LEN].copy_from_slice(&self.text);
        song[SONG_LEN - 2] = self.number;
        song[SONG_LEN - 1] = self.disc;
        song
    }
    pub fn name(&self) -> &str {
        let end = self.text.iter().position(|&c| c == b'\0').unwrap();
        unsafe { from_utf8_unchecked(&self.text[..end]) }
    }
    pub fn album(&self) -> &str {
        let mut start = 0;
        for (i, c) in self.text.into_iter().enumerate() {
            if c == b'\0' {
                if start == 0 {
                    start = i + 1;
                } else {
                    return unsafe { from_utf8_unchecked(&self.text[start..i]) };
                }
            }
        }
        unreachable!();
    }
    pub fn artist(&self) -> &str {
        let mut pos = [None; 2];
        for (i, c) in self.text.into_iter().enumerate() {
            if c == b'\0' {
                if pos[0].is_none() {
                    pos[0] = Some(i);
                } else if pos[1].is_none() {
                    pos[1] = Some(i);
                } else {
                    return unsafe { from_utf8_unchecked(&self.text[pos[1].unwrap() + 1..i]) };
                }
            }
        }
        unreachable!();
    }
    pub fn path(&self) -> &str {
        let mut pos = [None; 3];
        for (i, c) in self.text.into_iter().enumerate() {
            if c == b'\0' {
                if pos[0].is_none() {
                    pos[0] = Some(i);
                } else if pos[1].is_none() {
                    pos[1] = Some(i);
                } else if pos[2].is_none() {
                    pos[2] = Some(i);
                } else {
                    return unsafe { from_utf8_unchecked(&self.text[pos[2].unwrap() + 1..i]) };
                }
            }
        }
        unreachable!();
    }
}

impl From<&'_ [u8]> for Song {
    fn from(bytes: &[u8]) -> Self {
        Self {
            text: bytes[..TEXT_LEN].try_into().unwrap(),
            number: bytes[SONG_LEN - 2],
            disc: bytes[SONG_LEN - 1],
        }
    }
}

impl From<&'_ Path> for Song {
    fn from(path: &'_ Path) -> Self {
        let file = Box::new(File::open(path).expect("Could not open file."));
        let mss = MediaSourceStream::new(file, MediaSourceStreamOptions::default());

        let mut probe = match get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(probe) => probe,
            Err(e) => {
                panic!("{:?}", path);
            }
        };

        let mut name = String::from("Unknown Title");
        let mut album = String::from("Unknown Album");
        let mut artist = String::from("Unknown Artist");
        let mut number = 1;
        let mut disc = 1;

        let mut update_metadata = |metadata: &MetadataRevision| {
            for tag in metadata.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        StandardTagKey::AlbumArtist => artist = tag.value.to_string(),
                        StandardTagKey::Artist if artist == "Unknown Artist" => {
                            artist = tag.value.to_string()
                        }
                        StandardTagKey::Album => album = tag.value.to_string(),
                        StandardTagKey::TrackTitle => name = tag.value.to_string(),
                        StandardTagKey::TrackNumber => {
                            let num = tag.value.to_string();
                            if let Some((num, _)) = num.split_once('/') {
                                number = num.parse().unwrap_or(1);
                            } else {
                                number = num.parse().unwrap_or(1);
                            }
                        }
                        StandardTagKey::DiscNumber => {
                            let num = tag.value.to_string();
                            if let Some((num, _)) = num.split_once('/') {
                                disc = num.parse().unwrap_or(1);
                            } else {
                                disc = num.parse().unwrap_or(1);
                            }
                        }
                        _ => (),
                    }
                }
            }
        };

        //Why are there two different ways to get metadata?
        if let Some(metadata) = probe.metadata.get() {
            if let Some(current) = metadata.current() {
                update_metadata(current);
            }
        } else if let Some(metadata) = probe.format.metadata().current() {
            update_metadata(metadata);
        }

        Song::new(
            &name,
            &album,
            &artist,
            &path.to_string_lossy(),
            number,
            disc,
        )
    }
}

fn strip_padding(bytes: &[u8]) -> &str {
    for (i, b) in bytes.iter().enumerate() {
        if b == &b'\0' {
            unsafe {
                return from_utf8_unchecked(&bytes[..i]);
            }
        }
    }
    unreachable!();
}

struct Database {
    mmap: Mmap,
}

impl Database {
    pub fn new() -> Self {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open("db")
            .unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };
        Self { mmap }
    }
    pub fn get(&self, index: usize) -> Option<Song> {
        let start = SONG_LEN * index;
        let bytes = self.mmap.get(start..start + SONG_LEN)?;
        Some(Song::from(bytes))
    }
    pub fn len(&self) -> usize {
        self.mmap.len() / SONG_LEN
    }
}

fn create_db() {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("db")
        .unwrap();
    let mut writer = BufWriter::new(file);

    let paths: Vec<DirEntry> = walkdir::WalkDir::new("D:\\OneDrive\\Music")
        .into_iter()
        .flatten()
        .filter(|path| match path.path().extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
            }
            None => false,
        })
        .collect();

    let songs: Vec<Song> = paths
        .into_par_iter()
        .map(|path| Song::from(path.path()))
        .collect();

    for song in songs {
        writer.write_all(&song.into_bytes()).unwrap();
    }

    writer.flush().unwrap();
}

fn create_test_db() {
    let song = Song::new(
        "joe's song",
        "joe's album",
        "joe",
        "D:\\OneDrive\\Joe\\joe's song.flac",
        2,
        1,
    );
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("db")
        .unwrap();
    let mut writer = BufWriter::new(file);
    let bytes = song.into_bytes();
    for _ in 0..100_000 {
        writer.write_all(&bytes).unwrap();
    }
}

fn main() {
    let db = Database::new();
    let song = db.get(102).unwrap();
    let item = song.path();
    dbg!(item);
}

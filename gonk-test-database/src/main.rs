#![allow(unused)]
use std::{
    fmt::{Debug, Display},
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
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

const PAD_LEN: usize = 14;
const STR_LEN: usize = 512;
const SONG_LEN: usize = STR_LEN * 4 + size_of::<u8>() * 2 + PAD_LEN;

const NAME: Range<usize> = 0..STR_LEN;
const ALBUM: Range<usize> = STR_LEN..STR_LEN * 2;
const ARTIST: Range<usize> = STR_LEN * 2..STR_LEN * 3;
const PATH: Range<usize> = STR_LEN * 3..STR_LEN * 4;
const NUMBER: usize = SONG_LEN - PAD_LEN - 2;
const DISC: usize = SONG_LEN - PAD_LEN - 1;
const PAD: Range<usize> = SONG_LEN - PAD_LEN..SONG_LEN;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
struct Song {
    name: StaticStr,
    album: StaticStr,
    artist: StaticStr,
    path: StaticStr,
    number: u8,
    disc: u8,
    pad: [u8; PAD_LEN],
}

impl Song {
    pub fn into_bytes(self) -> [u8; SONG_LEN] {
        let mut bytes = [0; SONG_LEN];
        bytes[NAME].copy_from_slice(&self.name.0[..]);
        bytes[ALBUM].copy_from_slice(&self.album.0[..]);
        bytes[ARTIST].copy_from_slice(&self.artist.0[..]);
        bytes[PATH].copy_from_slice(&self.path.0[..]);
        bytes[NUMBER] = self.number;
        bytes[DISC] = self.disc;
        bytes[PAD].copy_from_slice(&[0; PAD_LEN]);
        bytes
    }
}

impl From<&'_ [u8]> for Song {
    fn from(from: &[u8]) -> Self {
        Self {
            name: StaticStr(from[NAME].try_into().unwrap()),
            album: StaticStr(from[ALBUM].try_into().unwrap()),
            artist: StaticStr(from[ARTIST].try_into().unwrap()),
            path: StaticStr(from[PATH].try_into().unwrap()),
            number: from[NUMBER],
            disc: from[DISC],
            pad: [0; PAD_LEN],
        }
    }
}

impl From<&'_ Path> for Song {
    fn from(from: &'_ Path) -> Self {
        let file = Box::new(File::open(from).expect("Could not open file."));
        let mss = MediaSourceStream::new(file, MediaSourceStreamOptions::default());

        let mut probe = match get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(probe) => probe,
            Err(e) => {
                panic!("{:?}", from);
            }
        };

        let mut song = Song {
            name: StaticStr::from("Unknown Title"),
            disc: 1,
            number: 1,
            path: StaticStr::from(from.to_str().unwrap()),
            album: StaticStr::from("Unknown Album"),
            artist: StaticStr::from("Unknown Artist"),
            pad: [0; PAD_LEN],
        };

        let mut update_metadata = |metadata: &MetadataRevision| {
            for tag in metadata.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        StandardTagKey::AlbumArtist => {
                            song.artist = StaticStr::from(tag.value.to_string().as_str())
                        }
                        StandardTagKey::Artist
                            if song.artist == StaticStr::from("Unknown Artist") =>
                        {
                            song.artist = StaticStr::from(tag.value.to_string().as_str())
                        }
                        StandardTagKey::Album => {
                            song.album = StaticStr::from(tag.value.to_string().as_str())
                        }
                        StandardTagKey::TrackTitle => {
                            song.name = StaticStr::from(tag.value.to_string().as_str())
                        }
                        StandardTagKey::TrackNumber => {
                            let number = tag.value.to_string();
                            if let Some((num, _)) = number.split_once('/') {
                                song.number = num.parse().unwrap_or(1);
                            } else {
                                song.number = number.parse().unwrap_or(1);
                            }
                        }
                        StandardTagKey::DiscNumber => {
                            let number = tag.value.to_string();
                            if let Some((num, _)) = number.split_once('/') {
                                song.disc = num.parse().unwrap_or(1);
                            } else {
                                song.disc = number.parse().unwrap_or(1);
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

        song
    }
}

impl Debug for Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Song")
            .field("name", &self.name)
            .field("album", &self.album)
            .field("artist", &self.artist)
            .field("path", &self.path)
            .field("number", &self.number)
            .field("disc", &self.disc)
            .finish()
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
struct StaticStr([u8; STR_LEN]);

impl From<&str> for StaticStr {
    fn from(from: &str) -> Self {
        if from.len() > STR_LEN {
            panic!("{} is '{} characters to big", from, from.len() - STR_LEN);
        }

        let mut array: [u8; STR_LEN] = [0; STR_LEN];
        for (i, b) in from.as_bytes().iter().enumerate() {
            array[i] = *b;
        }

        StaticStr(array)
    }
}

impl Debug for StaticStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StaticStr")
            .field(&format_args!("{}", self))
            .finish()
    }
}

impl Display for StaticStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let str = from_utf8_unchecked(&self.0);
            f.write_str(str)
        }
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
    pub fn songs_from_ids(&self, ids: &[usize]) -> Vec<Song> {
        let mut i = 0;
        let mut songs = Vec::new();
        for id in ids {
            match self.mmap.get(id * SONG_LEN..id * SONG_LEN + SONG_LEN) {
                Some(bytes) => songs.push(Song::from(bytes)),
                None => return songs,
            }
        }
        songs
    }
    pub fn songs(&self) -> Vec<Song> {
        let mut i = 0;
        let mut songs = Vec::new();
        loop {
            match self.mmap.get(i..i + SONG_LEN) {
                Some(bytes) => songs.push(Song::from(bytes)),
                None => return songs,
            }
            i += SONG_LEN;
        }
    }
    pub fn names_by_artist(&self, artist: &str) -> Vec<StaticStr> {
        self.query(artist.as_bytes(), ARTIST, NAME)
    }
    pub fn albums_by_artist(&self, artist: &str) -> Vec<StaticStr> {
        self.query(artist.as_bytes(), ARTIST, ALBUM)
    }
    pub fn query(
        &self,
        input: &[u8],
        query: Range<usize>,
        response: Range<usize>,
    ) -> Vec<StaticStr> {
        let mut i = 0;
        let mut items = Vec::new();
        loop {
            match self.mmap.get(i + query.start..i + query.end) {
                Some(query) => {
                    if query.starts_with(input) {
                        let response = self.mmap.get(i + response.start..i + response.end).unwrap();
                        items.push(StaticStr::from(strip_padding(response)));
                    }
                }
                None => return items,
            }
            i += SONG_LEN;
        }
    }
    pub fn names(&self) -> Vec<String> {
        self.collect(0)
    }
    pub fn albums(&self) -> Vec<String> {
        self.collect(1)
    }
    pub fn artists(&self) -> Vec<String> {
        self.collect(2)
    }
    pub fn paths(&self) -> Vec<String> {
        self.collect(3)
    }
    fn collect(&self, position: usize) -> Vec<String> {
        let mut i = 0;
        let mut names = Vec::new();
        let offset = STR_LEN * position;
        loop {
            match self.mmap.get(i + offset..i + offset + STR_LEN) {
                Some(name) => {
                    //Make sure to exclude the zero padding.
                    for (i, b) in name.iter().enumerate() {
                        if b == &b'\0' {
                            unsafe {
                                names.push(from_utf8_unchecked(&name[0..i]).to_string());
                            }
                            break;
                        }
                    }
                    i += SONG_LEN;
                }
                None => return names,
            }
        }
    }
}

fn write_db() {
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

fn main() {
    // let now = Instant::now();
    // write_db();
    // dbg!(now.elapsed());

    let db = Database::new();
    let now = Instant::now();
    let songs = db.albums_by_artist("Kendrick Lamar");
    dbg!(now.elapsed());
    dbg!(songs.len());
}

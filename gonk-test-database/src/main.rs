#![allow(unused)]
use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap},
    fmt::{Debug, Display},
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
    ops::Range,
    path::Path,
    str::{from_utf8, from_utf8_unchecked},
    time::Instant,
};

use memmap2::Mmap;

const PAD_LEN: usize = 14;
const STR_LEN: usize = 128;
const SONG_LEN: usize = STR_LEN * 4 + size_of::<u8>() * 2 + PAD_LEN;

const NAME: Range<usize> = (0..STR_LEN);
const ALBUM: Range<usize> = (STR_LEN..STR_LEN * 2);
const ARTIST: Range<usize> = (STR_LEN * 2..STR_LEN * 3);
const PATH: Range<usize> = (STR_LEN * 3..STR_LEN * 4);
const NUMBER: usize = SONG_LEN - PAD_LEN - 2;
const DISC: usize = SONG_LEN - PAD_LEN - 1;
const PAD: Range<usize> = (SONG_LEN - PAD_LEN..SONG_LEN);

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

impl<'a> From<&'a [u8]> for Song {
    fn from(from: &'a [u8]) -> Self {
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
            panic!("{} is greater than {} characters", from, STR_LEN);
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
fn strip_padding(bytes: &[u8]) -> String {
    for (i, b) in bytes.iter().enumerate() {
        if b == &b'\0' {
            unsafe {
                return from_utf8_unchecked(&bytes[..i]).to_string();
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
        let mut file = OpenOptions::new()
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
    pub fn names_by_artist(&self, artist: &str) -> Vec<String> {
        self.query(artist.as_bytes(), ARTIST, NAME)
    }
    pub fn albums_by_artist(&self, artist: &str) -> Vec<String> {
        self.query(artist.as_bytes(), ARTIST, ALBUM)
    }
    pub fn query(&self, input: &[u8], query: Range<usize>, response: Range<usize>) -> Vec<String> {
        let mut i = 0;
        let mut items = Vec::new();
        loop {
            match self.mmap.get(i + query.start..i + query.end) {
                Some(artist) => {
                    if artist.starts_with(input) {
                        let album = self.mmap.get(i + response.start..i + response.end).unwrap();
                        items.push(strip_padding(album));
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

fn write_db(file: &File) {
    let mut writer = BufWriter::new(file);

    let song = Song {
        name: StaticStr::from("joe's song"),
        album: StaticStr::from("joe's album"),
        artist: StaticStr::from("joe"),
        path: StaticStr::from("joe's path"),
        number: 50,
        disc: 100,
        pad: [0; PAD_LEN],
    };

    for i in 0..100_000 {
        let song = Song {
            name: StaticStr::from(format!("joe's song {}", i).as_str()),
            album: StaticStr::from("joe's album"),
            artist: StaticStr::from("joe"),
            path: StaticStr::from("joe's path"),
            number: 50,
            disc: 100,
            pad: [0; PAD_LEN],
        };
        writer.write_all(&song.into_bytes());
    }

    writer.flush().unwrap();
}

fn main() {
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("db")
        .unwrap();
    write_db(&file);
    drop(file);

    let db = Database::new();
    let now = Instant::now();
    let artits = db.albums_by_artist("joe");
    dbg!(artits.len());
    dbg!(now.elapsed());
}

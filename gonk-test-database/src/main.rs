#![allow(unused)]
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
    ops::Range,
    path::Path,
    time::Instant,
};

use memmap2::Mmap;

const STR_LEN: usize = 128;
const SONG_LEN: usize = STR_LEN * 4 + size_of::<u8>() * 2;

const NAME: Range<usize> = (0..STR_LEN);
const ALBUM: Range<usize> = (STR_LEN..STR_LEN * 2);
const ARTIST: Range<usize> = (STR_LEN * 2..STR_LEN * 3);
const PATH: Range<usize> = (STR_LEN * 3..STR_LEN * 4);
const NUMBER: usize = SONG_LEN - 2;
const DISC: usize = SONG_LEN - 1;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
struct Song {
    name: StaticStr,
    album: StaticStr,
    artist: StaticStr,
    path: StaticStr,
    number: u8,
    disc: u8,
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
        }
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

impl Display for StaticStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let str = std::str::from_utf8_unchecked(&self.0);
            f.write_str(str)
        }
    }
}

fn main() {
    let mut db = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("db")
        .unwrap();
    let mut writer = BufWriter::new(&db);

    let song = Song {
        name: StaticStr::from("joe's song"),
        album: StaticStr::from("joe's album"),
        artist: StaticStr::from("joe"),
        path: StaticStr::from("joe's path"),
        number: 50,
        disc: 100,
    };

    let bytes = song.into_bytes();
    let mut tree: BTreeMap<usize, usize> = BTreeMap::new();
    let mut pos = 0;

    for _ in 0..100_000 {
        tree.insert(tree.len(), pos);
        writer.write_all(&bytes);
        pos += SONG_LEN;
    }

    writer.flush().unwrap();

    let map = unsafe { Mmap::map(&db).unwrap() };

    let now = Instant::now();
    let start = *tree.get(&1000).unwrap();
    let end = *tree.get(&1001).unwrap();

    let song_bytes = &map[start..end];
    let song = Song::from(song_bytes);
    dbg!(now.elapsed());
    println!("{}", song.album);
}

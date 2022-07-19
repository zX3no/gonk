#![allow(unused)]
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Debug, Display},
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
    ops::Range,
    path::Path,
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
            let str = std::str::from_utf8_unchecked(&self.0);
            f.write_str(str)
        }
    }
}

fn read_vec() -> Vec<u64> {
    let file = File::open("tree").unwrap();
    let map = unsafe { Mmap::map(&file).unwrap() };
    let size = size_of::<u64>();
    let mut pos = 0;
    let mut vec = Vec::new();

    loop {
        match map.get(pos..pos + size) {
            Some(v) => {
                let v = u64::from_le_bytes(v.try_into().unwrap());
                vec.push(v);

                pos += size;
            }
            None => return vec,
        }
    }
}

fn write_vec(vec: Vec<u64>) {
    let mut tree = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("tree")
        .unwrap();

    let mut writer = BufWriter::new(&tree);

    for v in vec {
        writer.write_all(&v.to_le_bytes());
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

    let bytes = song.into_bytes();
    //TODO: maybe use u16 instead of u64
    //most must databases are not even close to 65,000 songs.
    let mut vec = Vec::new();
    let mut pos = 0;

    for _ in 0..100_000 {
        vec.push(pos);
        writer.write_all(&bytes);
        pos += SONG_LEN as u64;
    }

    writer.flush().unwrap();
    write_vec(vec);
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
    let vec = read_vec();

    // let now = Instant::now();
    // let start = *vec.get(10000).unwrap() as usize;
    // let end = *vec.get(10001).unwrap() as usize;

    // let song_bytes = &map[start..end];
    // let song = Song::from(song_bytes);

    // dbg!(now.elapsed());
    // dbg!(song);
    // write_vec(vec);
}

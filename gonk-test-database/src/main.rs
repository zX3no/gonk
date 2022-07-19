#![allow(unused)]
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
    path::Path,
    time::Instant,
};

use memmap2::Mmap;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone)]
struct Song {
    name: String,
    album: String,
    artist: String,
    path: String,
    folder: String,
    number: u8,
    disc: u8,
}

impl Song {
    pub fn len(&self) -> usize {
        self.name.len()
            + self.album.len()
            + self.artist.len()
            + self.path.len()
            + self.folder.len()
            + u8::BITS as usize
        // + (size_of::<u8>() * 8)
    }
    pub fn into_bytes(mut self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(self.len());
        unsafe {
            bytes.append(self.name.as_mut_vec());
            bytes.push(b'\n');
            bytes.append(self.album.as_mut_vec());
            bytes.push(b'\n');
            bytes.append(self.artist.as_mut_vec());
            bytes.push(b'\n');
            bytes.append(self.path.as_mut_vec());
            bytes.push(b'\n');
            bytes.append(self.folder.as_mut_vec());
            bytes.push(b'\n');
            bytes.push(self.number);
            bytes.push(self.disc);
            bytes.push(b'\r');
        }
        bytes
    }
}

fn main() {
    let mut tree: BTreeMap<usize, usize> = BTreeMap::new();
    // let mut tree: HashMap<usize, usize> = HashMap::new();

    let mut song = Song {
        name: String::from("this is a very very long name"),
        album: String::from("album album album album album"),
        artist: String::from("artist artist artist arist artist"),
        path: String::from("D:\\OneDrive\\Music\\Joe\\joe's song 1.flac"),
        folder: String::from("D:\\OneDrive\\Music"),
        number: 1,
        disc: 1,
    };
    let bytes = song.into_bytes();

    let mut db = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("db")
        .unwrap();

    let mut writer = BufWriter::new(&db);

    tree.insert(0, 0);
    let mut pos = 0;
    for i in 0..100_000 {
        writer.write_all(&bytes).unwrap();
        pos += bytes.len();
        tree.insert(tree.len(), pos);
    }

    writer.flush().unwrap();

    let map = unsafe { Mmap::map(&db).unwrap() };

    let now = Instant::now();
    for i in 0..tree.len() - 1 {
        let start = *tree.get(&i).unwrap();
        let end = *tree.get(&(i + 1)).unwrap();
        let bytes = match map.get(start..end) {
            Some(bytes) => bytes,
            None => panic!("{} {} {}", start, end, i),
        };
        let mut new_lines = 0;
        let mut pos = 0;
        for (i, b) in bytes.iter().enumerate() {
            if b == &b'\n' {
                new_lines += 1;
                if new_lines == 4 {
                    pos = i + 1;
                }
            } else if new_lines == 5 {
                let path = &map.get(pos..i - 1).unwrap();
                unsafe {
                    let path = std::str::from_utf8_unchecked(path);
                }
                // panic!("{}", artist);
                break;
            }
        }
    }

    dbg!(now.elapsed());

    let now = Instant::now();
    let ids: Vec<(usize, usize)> = tree.into_iter().collect();

    let mut tree = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open("tree")
        .unwrap();

    let mut writer = BufWriter::new(&tree);

    for (k, v) in ids {
        writer.write_all(&k.to_le_bytes());
        writer.write_all(&v.to_le_bytes());
    }
    dbg!(now.elapsed());
}

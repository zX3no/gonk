mod database;
mod fastdb;

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use database::Song;
use jwalk::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::fastdb::FastDB;

fn main() {
    let now = Instant::now();
    FastDB::write_min();
    println!("{:?}", now.elapsed());
    let now = Instant::now();
    let data = FastDB::read();
    let mut artists: Vec<_> = data.iter().step_by(3).collect();
    artists.sort_by_key(|artist| artist.to_lowercase());
    artists.dedup();
    dbg!(artists);
    println!("{:?}", now.elapsed());
}

fn test() {
    let now = Instant::now();

    //10ms
    let paths: Vec<PathBuf> = WalkDir::new(r"D:\OneDrive\Music")
        .into_iter()
        .filter_map(|entry| {
            if let Some(ex) = entry.as_ref().unwrap().path().extension() {
                if ex == "flac" || ex == "mp3" || ex == "m4a" || ex == "wav" {
                    return Some(entry.as_ref().unwrap().path());
                }
            }
            None
        })
        .collect();

    //450 ms
    let songs: Vec<Song> = paths
        .par_iter()
        .map(|path| Song::from(path.to_str().unwrap()))
        .collect();

    //200us
    let mut artist: Vec<String> = songs
        .par_iter()
        .map(|song| song.album_artist.clone())
        .collect();

    //500us
    artist.sort_by_key(|artist| artist.to_lowercase());

    //120us
    artist.dedup();

    println!("{:?}", now.elapsed());
}

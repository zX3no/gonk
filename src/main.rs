use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

use itertools::Itertools;
use jwalk::WalkDir;

mod database;
use database::song::Song;

fn main() {
    let path = Path::new(r"D:\OneDrive\Music\BadBadNotGood");

    let mut songs = Vec::new();

    for entry in WalkDir::new(path) {
        if let Ok(e) = entry {
            if let Some(ex) = e.path().extension() {
                if ex == "flac" {
                    let song = Song::from(e.path());

                    songs.push(song);
                }
            }
        }
    }

    let mut albums = Vec::new();

    for song in songs {
        albums.push(song.album);
    }

    let albums = albums.into_iter().unique().collect_vec();

    dbg!(albums);
}

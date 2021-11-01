use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::PathBuf,
    time::Instant,
};

use audiotags::Tag;
use jwalk::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::database::Song;

pub struct FastDB {}
impl FastDB {
    pub fn scan(path: &str) -> Vec<PathBuf> {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| {
                if let Some(ex) = entry.as_ref().unwrap().path().extension() {
                    if ex == "flac" || ex == "mp3" || ex == "m4a" {
                        return Some(entry.as_ref().unwrap().path());
                    }
                }
                None
            })
            .collect()
    }
    pub fn read() -> Vec<String> {
        let file = File::open("f.db").unwrap();
        let reader = BufReader::new(&file);

        reader.lines().flatten().collect()
    }
    pub fn write_min() {
        let p = "D:/OneDrive/Music";
        let paths = FastDB::scan(p);

        let songs: Vec<MinSong> = paths
            .par_iter()
            .map(|path| MinSong::from(path.to_str().unwrap()))
            .collect();

        let file = File::create("f.db").unwrap();
        let mut writer = BufWriter::new(&file);

        for s in songs {
            writer.write(s.artist.as_bytes()).unwrap();
            writer.write("\n".as_bytes()).unwrap();
            writer.write(s.path.to_str().unwrap().as_bytes()).unwrap();
            writer.write("\n".as_bytes()).unwrap();
            writer.write(s.album.as_bytes()).unwrap();
            writer.write("\n".as_bytes()).unwrap();
        }
    }
}
pub struct MinSong {
    path: PathBuf,
    album: String,
    artist: String,
}

impl MinSong {
    pub fn from(path: &str) -> Self {
        //this is slow
        if let Ok(tag) = Tag::new().read_from_path(&path) {
            let artist = if let Some(artist) = tag.album_artist() {
                artist.to_string()
            } else if let Some(artist) = tag.artist() {
                artist.to_string()
            } else {
                panic!("no artist for {:?}", path);
            };
            return MinSong {
                album: tag.album_title().unwrap().to_string(),
                artist,
                path: PathBuf::from(path),
            };
        }
        panic!();
    }
}

use crate::{
    database_path, log, profiler, reset, save_settings, settings_path, validate, Settings, Song,
    MMAP, SETTINGS, SONG_LEN,
};
use memmap2::Mmap;
use multimap::MultiMap;
use once_cell::unsync::Lazy;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fs::{self, OpenOptions};

//STATICS:

//I don't like this being an option :?
// pub static mut MMAP: Option<Mmap> = None;

//It's probably unnecessary to have settings as a static.
// pub static mut SETTINGS: Settings = Settings::default();

//TODO: Maybe change this to once?
pub static mut DB: Lazy<Database> = Lazy::new(|| unsafe { Database::new() });

pub struct Database {
    data: MultiMap<String, Album>,
}

impl Database {
    //This should replace gonk_core::init();
    pub unsafe fn new() -> Self {
        profiler::init();

        let mut data = MultiMap::new();

        match fs::read(settings_path()) {
            Ok(bytes) if !bytes.is_empty() => match Settings::from(bytes) {
                Some(settings) => SETTINGS = settings,
                None => save_settings(),
            },
            //Save the default settings if nothing is found.
            _ => save_settings(),
        }

        //We only need write access to create the file.
        let db = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(database_path())
            .unwrap();

        let mmap = Mmap::map(&db).unwrap();

        //Reset the database if the first song is invalid.
        if validate(&mmap).is_err() {
            drop(mmap);
            log!("Database is corrupted. Resetting!");
            reset().unwrap();
        } else {
            //Load all songs into memory.
            let songs: Vec<Song> = (0..mmap.len() / SONG_LEN)
                .into_par_iter()
                .map(|i| {
                    let pos = i * SONG_LEN;
                    let bytes = &mmap[pos..pos + SONG_LEN];
                    Song::from(bytes, i)
                })
                .collect();

            let mut albums = MultiMap::new();
            for song in songs {
                albums.insert(
                    (song.artist, song.album),
                    MinimalSong {
                        title: song.title,
                        track_number: song.number,
                        disc_number: song.disc,
                        path: song.path,
                        gain: song.gain,
                    },
                )
            }

            for ((artist, album), v) in albums {
                data.insert(
                    artist,
                    Album {
                        title: album,
                        songs: v,
                    },
                );
            }
        }

        MMAP = Some(Mmap::map(&db).unwrap());

        Self { data }
    }

    ///Get all aritist names.
    pub fn artists() -> Vec<&'static String> {
        let db = unsafe { &DB.data };
        db.keys().collect()
    }

    ///Get all albums by an artist.
    pub fn albums(artist: &str) -> Vec<&'static Album> {
        let db = unsafe { &DB.data };
        for (ar, al) in db.iter_all() {
            if artist == ar {
                return al.iter().collect();
            }
        }
        Vec::new()
    }

    ///Get all albums names by an artist.
    pub fn album_names(artist: &str) -> Vec<&'static String> {
        Database::albums(artist)
            .iter()
            .map(|album| &album.title)
            .collect()
    }

    ///Get album by artist and album name.
    pub fn album(artist: &str, album: &str) -> Option<&'static Album> {
        let db = unsafe { &DB.data };
        for (ar, albums) in db.iter_all() {
            if artist == ar {
                for al in albums {
                    if album == al.title {
                        return Some(al);
                    }
                }
            }
        }
        None
    }
    //TODO: Replace Song with MinSong
    pub fn songs() -> Vec<&'static MinimalSong> {
        let db = unsafe { &DB.data };
        let mut songs = Vec::new();
        for (_, albums) in db.iter_all() {
            for album in albums {
                songs.extend(&album.songs)
            }
        }
        songs
    }
}

#[derive(Debug)]
pub struct Artist {
    pub albums: Vec<Album>,
}

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub songs: Vec<MinimalSong>,
}

//TODO: We only needs Song and RawSong.
#[derive(Debug)]
pub struct MinimalSong {
    pub title: String,
    pub track_number: u8,
    pub disc_number: u8,
    pub path: String,
    pub gain: f32,
}

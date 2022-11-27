use crate::{
    database_path, log, profile, reset, save_settings, settings_path, validate, Settings, SETTINGS,
    SONG_LEN,
};
use memmap2::Mmap;
use multimap::MultiMap;
use once_cell::unsync::Lazy;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fs::{self, OpenOptions};

//I don't like this being an option :?
// pub static mut MMAP: Option<Mmap> = None;

//It's probably unnecessary to have settings as a static.
// pub static mut SETTINGS: Settings = Settings::default();

//TODO: Maybe change this to a Once<_>?
pub static mut DB: Lazy<Database> = Lazy::new(|| unsafe { Database::new() });

pub struct Database {
    data: MultiMap<String, Album>,
}

impl Database {
    //This should replace gonk_core::init();
    pub unsafe fn new() -> Self {
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
            //TODO: Maybe do this on another thread.
            //Waiting could be quite costly for large libraries.

            //Load all songs into memory.
            let songs: Vec<crate::OldSong> = (0..mmap.len() / SONG_LEN)
                .into_par_iter()
                .map(|i| {
                    let pos = i * SONG_LEN;
                    let bytes = &mmap[pos..pos + SONG_LEN];
                    crate::OldSong::from(bytes, i)
                })
                .collect();

            let mut albums = MultiMap::new();

            //Add songs to albums.
            for song in songs {
                albums.insert(
                    (song.artist, song.album),
                    Song {
                        title: song.title,
                        disc_number: song.disc,
                        track_number: song.number,
                        path: song.path,
                        gain: song.gain,
                    },
                )
            }

            //Sort songs.
            for (_, album) in &mut albums {
                album.sort_unstable_by(|a, b| {
                    if a.disc_number == b.disc_number {
                        a.track_number.cmp(&b.track_number)
                    } else {
                        a.disc_number.cmp(&b.disc_number)
                    }
                });
            }

            //Add albums to artists.
            for ((artist, album), v) in albums {
                data.insert(
                    artist,
                    Album {
                        title: album,
                        songs: v,
                    },
                );
            }

            //Sort albums.
            for (_, albums) in &mut data {
                albums.sort_unstable_by_key(|album| album.title.to_ascii_lowercase());
            }
        }

        Self { data }
    }

    ///Browser queries.

    ///Get all aritist names.
    pub fn artists() -> Vec<&'static String> {
        let db = unsafe { &DB.data };
        //TODO: Can you sort keys?
        let mut v = Vec::from_iter(db.keys());
        v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
        v
    }

    ///Get all albums by an artist.
    pub fn albums_by_artist(artist: &str) -> Vec<&'static Album> {
        let db = unsafe { &DB.data };
        if let Some(albums) = db.get_vec(artist) {
            return albums.iter().collect();
        }
        Vec::new()
    }

    ///Search Queries

    pub fn raw() -> &'static MultiMap<String, Album> {
        unsafe { &DB.data }
    }

    ///

    ///Get albums by aritist.
    pub fn artist(artist: &str) -> Option<&'static Vec<Album>> {
        let db = unsafe { &DB.data };
        db.get_vec(artist)
    }

    ///Get all album names.
    pub fn albums() -> Vec<&'static String> {
        profile!();
        let db = unsafe { &DB.data };
        let mut albums = Vec::new();
        for (_, al) in db.iter_all() {
            for album in al {
                albums.push(&album.title);
            }
        }
        albums
    }

    ///Get all albums names by an artist.
    pub fn album_names_by_artist(artist: &str) -> Vec<&'static String> {
        Database::albums_by_artist(artist)
            .iter()
            .map(|album| &album.title)
            .collect()
    }

    ///Get album by artist and album name.
    pub fn album(artist: &str, album: &str) -> Option<&'static Album> {
        let db = unsafe { &DB.data };
        if let Some(albums) = db.get_vec(artist) {
            for al in albums {
                if album == al.title {
                    return Some(al);
                }
            }
        }
        None
    }

    ///Get all songs in the database.
    pub fn songs() -> Vec<&'static Song> {
        let db = unsafe { &DB.data };
        let mut songs = Vec::new();
        for (_, albums) in db.iter_all() {
            for album in albums {
                songs.extend(&album.songs)
            }
        }
        songs
    }

    ///Get an individual song in the database.
    pub fn song(artist: &str, album: &str, disc: u8, number: u8) -> Option<&'static Song> {
        profile!();
        let db = unsafe { &DB.data };

        if let Some(albums) = db.get_vec(artist) {
            for al in albums {
                if al.title == album {
                    for song in &al.songs {
                        if song.disc_number == disc && song.track_number == number {
                            return Some(song);
                        }
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct Artist {
    pub albums: Vec<Album>,
}

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

//TODO: Replace Song with MinimalSong.
#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub title: String,
    pub disc_number: u8,
    pub track_number: u8,
    pub path: String,
    pub gain: f32,
}

use crate::{
    database_path, log, reset, save_settings, settings_path, validate, Settings, SETTINGS, SONG_LEN,
};
use memmap2::Mmap;
use once_cell::unsync::Lazy;
use rayon::{
    prelude::{IntoParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
    fs::{self, OpenOptions},
};

//I don't like this being an option :?
// pub static mut MMAP: Option<Mmap> = None;

//It's probably unnecessary to have settings as a static.
// pub static mut SETTINGS: Settings = Settings::default();

//TODO: Maybe change this to a Once<_>?
pub static mut DB: Lazy<Database> = Lazy::new(|| unsafe { Database::new() });

pub struct Database {
    data: BTreeMap<String, Vec<Album>>,
}

impl Database {
    //This should replace gonk_core::init();
    pub unsafe fn new() -> Self {
        let mut data: BTreeMap<String, Vec<Album>> = BTreeMap::new();

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

            let mut albums: BTreeMap<(String, String), Vec<Song>> = BTreeMap::new();

            //Add songs to albums.
            for song in songs {
                let v = Song {
                    title: song.title,
                    disc_number: song.disc,
                    track_number: song.number,
                    path: song.path,
                    gain: song.gain,
                };
                match albums.entry((song.artist, song.album)) {
                    Entry::Occupied(mut entry) => entry.get_mut().push(v),
                    Entry::Vacant(entry) => {
                        entry.insert(vec![v]);
                    }
                }
            }

            //Sort songs.
            albums.iter_mut().for_each(|(_, album)| {
                album.sort_unstable_by(|a, b| {
                    if a.disc_number == b.disc_number {
                        a.track_number.cmp(&b.track_number)
                    } else {
                        a.disc_number.cmp(&b.disc_number)
                    }
                });
            });

            //Add albums to artists.
            for ((artist, album), v) in albums {
                let v = Album {
                    title: album,
                    songs: v,
                };
                match data.entry(artist) {
                    Entry::Occupied(mut entry) => entry.get_mut().push(v),
                    Entry::Vacant(entry) => {
                        entry.insert(vec![v]);
                    }
                }
            }

            //Sort albums.
            data.iter_mut().for_each(|(_, albums)| {
                albums.sort_unstable_by_key(|album| album.title.to_ascii_lowercase());
            });
        }

        Self { data }
    }

    //Browser Queries:

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
        if let Some(albums) = db.get(artist) {
            albums.iter().collect()
        } else {
            Vec::new()
        }
    }

    ///Get album by artist and album name.
    pub fn album(artist: &str, album: &str) -> Option<&'static Album> {
        let db = unsafe { &DB.data };
        if let Some(albums) = db.get(artist) {
            for al in albums {
                if album == al.title {
                    return Some(al);
                }
            }
        }
        None
    }

    ///Get an individual song in the database.
    pub fn song(artist: &str, album: &str, disc: u8, number: u8) -> Option<&'static Song> {
        let db = unsafe { &DB.data };

        if let Some(albums) = db.get(artist) {
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

    ///Get albums by aritist.
    pub fn artist(artist: &str) -> Option<&'static Vec<Album>> {
        let db = unsafe { &DB.data };
        db.get(artist)
    }

    //Search Queries:

    ///Search the database and return the 25 most accurate matches.
    pub fn search(query: &str) -> Vec<Item> {
        let db = unsafe { &DB.data };

        let query = query.to_lowercase();
        let mut results = Vec::new();

        //Calculate if the input string is close to the query.
        let mut cal = |input: Item| {
            let str = match input {
                Item::Artist(artist) => artist,
                Item::Album((album, _)) => album,
                Item::Song((song, _, _, _, _)) => song,
            };
            let acc = strsim::jaro_winkler(&query, &str.to_lowercase());
            if acc > MIN_ACCURACY {
                results.push((input, acc));
            }
        };

        for (artist, albums) in db.iter() {
            cal(Item::Artist(artist));
            for album in albums {
                cal(Item::Album((&album.title, artist)));
                for song in &album.songs {
                    cal(Item::Song((
                        &song.title,
                        &album.title,
                        artist,
                        song.disc_number,
                        song.track_number,
                    )));
                }
            }
        }

        //Sort results by score.
        results.par_sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        if results.len() > 25 {
            //Remove the less accurate results.
            results.drain(25..);
        }

        // dbg!(&results);

        //Sort songs with equal score. Artist > Album > Song.
        results.sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
            if score_1 == score_2 {
                match item_1 {
                    Item::Artist(_) => match item_2 {
                        Item::Song(_) | Item::Album(_) => Ordering::Less,
                        Item::Artist(_) => Ordering::Equal,
                    },
                    Item::Album(_) => match item_2 {
                        Item::Song(_) => Ordering::Less,
                        Item::Album(_) => Ordering::Equal,
                        Item::Artist(_) => Ordering::Greater,
                    },
                    Item::Song(_) => match item_2 {
                        Item::Song(_) => Ordering::Equal,
                        Item::Album(_) | Item::Artist(_) => Ordering::Greater,
                    },
                }
            } else if score_2 > score_1 {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        });

        results.into_iter().map(|(item, _)| item).collect()
    }

    //

    pub fn raw() -> &'static BTreeMap<String, Vec<Album>> {
        unsafe { &DB.data }
    }

    //Get all album names.
    // pub fn albums() -> Vec<&'static String> {
    //     let db = unsafe { &DB.data };
    //     let mut albums = Vec::new();
    //     for (_, al) in db.iter() {
    //         for album in al {
    //             albums.push(&album.title);
    //         }
    //     }
    //     albums
    // }

    //Get all albums names by an artist.
    // pub fn album_names_by_artist(artist: &str) -> Vec<&'static String> {
    //     Database::albums_by_artist(artist)
    //         .iter()
    //         .map(|album| &album.title)
    //         .collect()
    // }

    //Get all songs in the database.
    // pub fn songs() -> Vec<&'static Song> {
    //     let db = unsafe { &DB.data };
    //     let mut songs = Vec::new();
    //     for (_, albums) in db.iter() {
    //         for album in albums {
    //             songs.extend(&album.songs)
    //         }
    //     }
    //     songs
    // }
}

#[derive(Clone, Debug)]
pub enum Item {
    ///(Name, Album, Artist, Disc Number, Track Number)
    Song((&'static String, &'static String, &'static String, u8, u8)),
    ///(Album, Artist)
    Album((&'static String, &'static String)),
    ///(Artist)
    Artist(&'static String),
}

const MIN_ACCURACY: f64 = 0.70;

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

mod strsim {
    use std::cmp::{max, min};

    pub fn jaro_winkler(a: &str, b: &str) -> f64 {
        let jaro_distance = generic_jaro(a, b);

        // Don't limit the length of the common prefix
        let prefix_length = a
            .chars()
            .zip(b.chars())
            .take_while(|&(ref a_elem, ref b_elem)| a_elem == b_elem)
            .count();

        let jaro_winkler_distance =
            jaro_distance + (0.2 * prefix_length as f64 * (1.0 - jaro_distance));

        jaro_winkler_distance.clamp(0.0, 1.0)
    }

    pub fn generic_jaro(a: &str, b: &str) -> f64 {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        // The check for lengths of one here is to prevent integer overflow when
        // calculating the search range.
        if a_len == 0 && b_len == 0 {
            return 1.0;
        } else if a_len == 0 || b_len == 0 {
            return 0.0;
        } else if a_len == 1 && b_len == 1 {
            return if a.chars().eq(b.chars()) { 1.0 } else { 0.0 };
        }

        let search_range = (max(a_len, b_len) / 2) - 1;

        let mut b_consumed = vec![false; b_len];
        let mut matches = 0.0;

        let mut transpositions = 0.0;
        let mut b_match_index = 0;

        for (i, a_elem) in a.chars().enumerate() {
            let min_bound =
            // prevent integer wrapping
            if i > search_range {
                max(0, i - search_range)
            } else {
                0
            };

            let max_bound = min(b_len - 1, i + search_range);

            if min_bound > max_bound {
                continue;
            }

            for (j, b_elem) in b.chars().enumerate() {
                if min_bound <= j && j <= max_bound && a_elem == b_elem && !b_consumed[j] {
                    b_consumed[j] = true;
                    matches += 1.0;

                    if j < b_match_index {
                        transpositions += 1.0;
                    }
                    b_match_index = j;

                    break;
                }
            }
        }

        if matches == 0.0 {
            0.0
        } else {
            (1.0 / 3.0)
                * ((matches / a_len as f64)
                    + (matches / b_len as f64)
                    + ((matches - transpositions) / matches))
        }
    }
}

//! Virtual database
//!
//! Songs are taken from the physical database and stored in a `BTreeMap`
//!
//! Also contains code for querying artists, albums and songs.
//!
use crate::db::{Album, Artwork, Song};
use crate::{Deserialize, strsim};
use std::{cmp::Ordering, collections::BTreeMap, fs, path::Path, str::from_utf8_unchecked};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

const MIN_ACCURACY: f64 = 0.70;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    ///(Artist, Album, Name, Disc Number, Track Number)
    Song((String, String, String, u8, u8)),
    ///(Artist, Album)
    Album((String, String)),
    ///(Artist)
    Artist(String),
}

fn normalize_for_search(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

fn score_match(query: &str, text: &str) -> f64 {
    let text = normalize_for_search(text);
    if query.is_empty() || text.is_empty() {
        return 0.0;
    }
    if text == query {
        return 1.0;
    }

    if text.starts_with(query) {
        let coverage = query.len() as f64 / text.len() as f64;
        return 0.95 + 0.05 * coverage;
    }

    let mut best_fuzzy = strsim::jaro_winkler(query, &text);
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        if word == query {
            return 0.94;
        }
        if word.starts_with(query) {
            let coverage = query.len() as f64 / word.len() as f64;
            return 0.90 + 0.04 * coverage;
        }
        best_fuzzy = best_fuzzy.max(strsim::jaro_winkler(query, word));
    }

    if text.contains(query) {
        return 0.88;
    }

    best_fuzzy
}

fn jaro(query: &str, input: Item) -> Result<(Item, f64), (Item, f64)> {
    let text = match input {
        Item::Artist(ref artist) => artist.as_str(),
        Item::Album((_, ref album)) => album.as_str(),
        Item::Song((_, _, ref song, _, _)) => song.as_str(),
    };
    let acc = score_match(query, text);
    if acc > MIN_ACCURACY {
        Ok((input, acc))
    } else {
        Err((input, acc))
    }
}

fn item_rank(item: &Item) -> u8 {
    match item {
        Item::Artist(_) => 0,
        Item::Album(_) => 1,
        Item::Song(_) => 2,
    }
}

//I feel like Box<[String, Box<Album>]> might have been a better choice.
pub struct Database {
    pub btree: BTreeMap<String, Vec<Album>>,
    pub len: usize,
}

impl Database {
    ///Read the database from disk and load it into memory.
    pub fn new(database_path: &Path) -> Self {
        let bytes = match fs::read(database_path) {
            Ok(bytes) => bytes,
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => Vec::new(),
                _ => panic!("{error}"),
            },
        };
        let songs: Vec<Song> = unsafe { from_utf8_unchecked(&bytes) }
            .lines()
            .flat_map(Song::deserialize)
            .collect();

        let len = songs.len();
        let mut btree: BTreeMap<String, Vec<Album>> = BTreeMap::new();
        let mut albums: BTreeMap<(String, String), Vec<Song>> = BTreeMap::new();

        //Add songs to albums.
        for song in songs.into_iter() {
            albums
                .entry((song.artist.clone(), song.album.clone()))
                .or_default()
                .push(song);
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
        for ((artist, title), songs) in albums {
            btree
                .entry(artist)
                .or_default()
                .push(Album { title, songs });
        }

        //Sort albums.
        btree.iter_mut().for_each(|(_, albums)| {
            albums.sort_unstable_by_key(|album| album.title.to_ascii_lowercase());
        });

        Self { btree, len }
    }

    ///Get all artist names.
    pub fn artists(&self) -> Vec<&String> {
        let mut v: Vec<_> = self.btree.keys().collect();
        v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
        v
    }

    ///Get all albums by an artist.
    pub fn albums_by_artist(&self, artist: &str) -> &[Album] {
        self.btree.get(artist).unwrap()
    }

    ///Get an album by artist and album name.
    pub fn album(&self, artist: &str, album: &str) -> &Album {
        if let Some(albums) = self.btree.get(artist) {
            for al in albums {
                if album == al.title {
                    return al;
                }
            }
        }
        panic!("Could not find album {} {}", artist, album);
    }

    ///Get an individual song in the database.
    pub fn song(&self, artist: &str, album: &str, disc: u8, number: u8) -> &Song {
        for al in self.btree.get(artist).unwrap() {
            if al.title == album {
                for song in &al.songs {
                    if song.disc_number == disc && song.track_number == number {
                        return song;
                    }
                }
            }
        }
        unreachable!();
    }

    ///Search the database and return the 25 most accurate matches.
    pub fn search(&self, query: &str) -> Vec<Item> {
        const MAX: usize = 40;

        let query = normalize_for_search(query);
        let mut results = Vec::new();

        for (artist, albums) in self.btree.iter() {
            for album in albums.iter() {
                for song in album.songs.iter() {
                    results.push(jaro(
                        &query,
                        Item::Song((
                            song.artist.clone(),
                            song.album.clone(),
                            song.title.clone(),
                            song.disc_number,
                            song.track_number,
                        )),
                    ));
                }
                results.push(jaro(
                    &query,
                    Item::Album((artist.clone(), album.title.clone())),
                ));
            }
            results.push(jaro(&query, Item::Artist(artist.clone())));
        }

        if query.is_empty() {
            return results
                .into_iter()
                .take(MAX)
                .map(|item| match item {
                    Ok((item, _)) => item,
                    Err((item, _)) => item,
                })
                .collect();
        }

        let mut results: Vec<(Item, f64)> = results.into_iter().flatten().collect();

        // Highest score first, on ties prefer Artist > Album > Song, then track order.
        results.sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
            match score_2.partial_cmp(score_1).unwrap_or(Ordering::Equal) {
                Ordering::Equal => match item_rank(item_1).cmp(&item_rank(item_2)) {
                    Ordering::Equal => match (item_1, item_2) {
                        (
                            Item::Song((_, _, _, disc_a, number_a)),
                            Item::Song((_, _, _, disc_b, number_b)),
                        ) => disc_a.cmp(disc_b).then(number_a.cmp(number_b)),
                        _ => Ordering::Equal,
                    },
                    ord => ord,
                },
                ord => ord,
            }
        });

        if results.len() > MAX {
            results.truncate(MAX);
        }

        results.into_iter().map(|(item, _)| item).collect()
    }

    //TODO: Ahh, yeah, so this kind of lookup on every single song render is a bit questionable.
    pub fn artwork(&self, song: &Song) -> Option<(&[u32], usize, usize)> {
        let album = self.album(&song.artist, &song.album);
        if let Some(song) = album.songs.first()
            && let Some(artwork) = &song.artwork
        {
            return match artwork {
                Artwork::Decoded(pixels, width, height) => Some((pixels, *width, *height)),
                Artwork::Compressed(_) => None,
            };
        }

        None
    }
}

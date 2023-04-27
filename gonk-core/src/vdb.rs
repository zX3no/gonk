//! Virtual database
//!
//! Songs are taken from the physical database and stored in a `BTreeMap`
//!
//! Also contains code for querying artists, albums and songs.
//!
use crate::strsim;
use crate::{
    db::{Album, Song},
    profile,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;

const MIN_ACCURACY: f64 = 0.70;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    ///(Artist, Album, Name, Disc Number, Track Number)
    Song((&'static str, &'static str, &'static str, u8, u8)),
    ///(Artist, Album)
    Album((&'static str, &'static str)),
    ///(Artist)
    Artist(&'static str),
}

fn jaro(query: &str, input: Item) -> Result<(Item, f64), (Item, f64)> {
    let str = match input {
        Item::Artist(artist) => artist,
        Item::Album((_, album)) => album,
        Item::Song((_, _, song, _, _)) => song,
    };
    let acc = strsim::jaro_winkler(query, &str.to_lowercase());
    if acc > MIN_ACCURACY {
        Ok((input, acc))
    } else {
        Err((input, acc))
    }
}

static mut SONGS: Vec<Song> = Vec::new();
static mut BTREE: BTreeMap<&str, Vec<Album>> = BTreeMap::new();

pub fn create() {
    unsafe {
        SONGS = crate::db::read().unwrap();
        let mut albums: BTreeMap<(&str, &str), Vec<&Song>> = BTreeMap::new();

        //Add songs to albums.
        for song in &SONGS {
            albums
                .entry((song.artist.as_str(), song.album.as_str()))
                .or_insert_with(Vec::new)
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
            BTREE
                .entry(artist)
                .or_insert_with(Vec::new)
                .push(Album { title, songs });
        }

        //Sort albums.
        BTREE.iter_mut().for_each(|(_, albums)| {
            albums.sort_unstable_by_key(|album| album.title.to_ascii_lowercase());
        });
    }
}

pub fn artists() -> Vec<&'static str> {
    let mut v: Vec<_> = unsafe { &BTREE }.keys().map(|key| *key).collect();
    v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
    v
}

pub fn albums() -> Vec<(&'static str, &'static str)> {
    unsafe { &BTREE }
        .iter()
        .flat_map(|(k, v)| v.iter().map(|album| (*k, album.title)))
        .collect()
}

pub fn albums_by_artist(artist: &str) -> Vec<Vec<&'static Song>> {
    unsafe { &BTREE }
        .get(artist)
        .map(|albums| {
            albums
                .iter()
                .map(|album| album.songs.iter().map(|song| *song).collect())
                .collect()
        })
        .unwrap_or_else(Vec::new)
}

pub fn album(artist: &str, album: &str) -> &'static Album {
    if let Some(albums) = unsafe { &BTREE }.get(artist) {
        for al in albums {
            if album == al.title {
                return al;
            }
        }
    }
    panic!("Could not find album {} {}", artist, album);
}

pub fn song(artist: &str, album: &str, disc: u8, number: u8) -> Option<&'static Song> {
    profile!();
    if let Some(albums) = unsafe { &BTREE }.get(artist) {
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

pub fn search(query: &str) -> Vec<Item> {
    let query = query.to_lowercase();

    let mut results = Vec::new();

    for (artist, albums) in unsafe { &BTREE }.iter() {
        for album in albums.iter() {
            for song in album.songs.iter() {
                results.push(jaro(
                    &query,
                    Item::Song((
                        &song.artist,
                        &song.album,
                        &song.title,
                        song.disc_number,
                        song.track_number,
                    )),
                ));
            }
            results.push(jaro(&query, Item::Album((artist, album.title))));
        }
        results.push(jaro(&query, Item::Artist(artist)));
    }

    let mut results: Vec<(Item, f64)> = results.into_iter().flatten().collect();

    if !query.is_empty() {
        //Sort results by score.
        results.sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
    }

    if results.len() > 40 {
        //Remove the less accurate results.
        unsafe {
            results.set_len(40);
        }
    }

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
                Item::Song((_, _, _, disc_a, number_a)) => match item_2 {
                    Item::Song((_, _, _, disc_b, number_b)) => match disc_a.cmp(disc_b) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Equal => number_a.cmp(number_b),
                        Ordering::Greater => Ordering::Greater,
                    },
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

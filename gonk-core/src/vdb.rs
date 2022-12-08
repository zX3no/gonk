//! Virtual database
//!
//! Songs are taken from the physical database and stored in a `BTreeMap`
//!
//! Also contains code for querying artists, albums and songs.
//!
use crate::*;
use rayon::{
    prelude::{IntoParallelRefIterator, ParallelDrainRange, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
    error::Error,
    sync::RwLock,
};
pub type Database = BTreeMap<String, Vec<Album>>;

pub fn create() -> Result<Database, Box<dyn Error + Send + Sync>> {
    let songs = db::read()?;
    let mut data: BTreeMap<String, Vec<Album>> = BTreeMap::new();
    let mut albums: BTreeMap<(String, String), Vec<Song>> = BTreeMap::new();

    //Add songs to albums.
    for song in songs {
        match albums.entry((song.artist.clone(), song.album.clone())) {
            Entry::Occupied(mut entry) => entry.get_mut().push(song),
            Entry::Vacant(entry) => {
                entry.insert(vec![song]);
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

    Ok(data as Database)
}

//Browser Queries:

///Get all aritist names.
pub fn artists(db: &Database) -> Vec<&String> {
    let mut v = Vec::from_iter(db.keys());
    v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
    v
}

///Get all albums by an artist.
pub fn albums_by_artist(db: &'static Database, artist: &str) -> Option<&'static [Album]> {
    match db.get(artist) {
        Some(albums) => Some(albums.as_slice()),
        None => None,
    }
}

///Get album by artist and album name.
pub fn album(db: &'static Database, artist: &str, album: &str) -> Option<&'static Album> {
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
pub fn song<'a>(
    db: &'a Database,
    artist: &str,
    album: &str,
    disc: u8,
    number: u8,
) -> Option<&'a Song> {
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
pub fn artist<'a>(db: &'a Database, artist: &str) -> Option<&'a Vec<Album>> {
    db.get(artist)
}

#[derive(Clone, Debug)]
pub enum Item {
    ///(Artist, Album, Name, Disc Number, Track Number)
    Song((&'static String, &'static String, &'static String, u8, u8)),
    ///(Artist, Album)
    Album((&'static String, &'static String)),
    ///(Artist)
    Artist(&'static String),
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

///Search the database and return the 25 most accurate matches.
pub fn search(db: &'static Database, query: &str) -> Vec<Item> {
    let query = query.to_lowercase();
    let results = RwLock::new(Vec::new());

    let iter = db.par_iter();

    iter.for_each(|(artist, albums)| {
        for album in albums {
            for song in &album.songs {
                let song = jaro(
                    &query,
                    Item::Song((
                        artist,
                        &album.title,
                        &song.title,
                        song.disc_number,
                        song.track_number,
                    )),
                );
                results.write().unwrap().push(song);
            }
            let album = jaro(&query, Item::Album((artist, &album.title)));
            results.write().unwrap().push(album);
        }
        let artist = jaro(&query, Item::Artist(artist));
        results.write().unwrap().push(artist);
    });

    let results = RwLock::into_inner(results).unwrap();

    let mut results: Vec<_> = if query.is_empty() {
        results
            .into_iter()
            .take(25)
            .filter_map(|x| match x {
                Ok(_) => None,
                Err(x) => Some(x),
            })
            .collect()
    } else {
        results.into_iter().flatten().collect()
    };

    //Sort results by score.
    results.par_sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

    if results.len() > 25 {
        //Remove the less accurate results.
        results.par_drain(25..);
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

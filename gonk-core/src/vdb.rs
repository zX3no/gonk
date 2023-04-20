//! Virtual database
//!
//! Songs are taken from the physical database and stored in a `BTreeMap`
//!
//! Also contains code for querying artists, albums and songs.
//!
use crate::db::{Album, Song};
use crate::profile;
use crate::strsim;
use itertools::Itertools;
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

const MIN_ACCURACY: f64 = 0.70;

#[derive(Clone, Debug)]
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

trait VDB {
    ///Get all aritist names.
    fn artists(&self) -> Vec<&'static str>;

    ///Get all album names.
    fn albums(&self) -> Vec<(&'static str, &'static str)>;

    ///Get all albums by an artist.
    fn albums_by_artist(&self, artist: &str) -> Vec<Vec<&Song>>;

    ///Get album by artist and album name.
    fn album(&self, artist: &str, album: &str) -> Vec<&Song>;

    ///Get an individual song in the database.
    fn song(&self, artist: &str, album: &str, disc: u8, number: u8) -> Option<&Song>;

    ///Search the database and return the 25 most accurate matches.
    fn search(&self, query: &str) -> Vec<Item>;
}

impl VDB for &'static [Song] {
    fn artists(&self) -> Vec<&'static str> {
        let mut artists = self
            .iter()
            .map(|song| song.artist.as_str())
            .collect::<Vec<&str>>();
        artists.dedup();
        artists
    }

    fn albums(&self) -> Vec<(&'static str, &'static str)> {
        let mut albums: Vec<_> = self
            .iter()
            .map(|song| (song.artist.as_str(), song.album.as_str()))
            .collect();
        albums.dedup();
        albums
    }

    fn albums_by_artist(&self, artist: &str) -> Vec<Vec<&Song>> {
        self.iter()
            .filter(|song| song.artist == artist)
            .sorted_by_key(|song| &song.album)
            .group_by(|song| &song.album)
            .into_iter()
            .map(|(_, group)| group.collect())
            .collect::<Vec<Vec<&Song>>>()
    }

    fn album(&self, artist: &str, album: &str) -> Vec<&Song> {
        self.iter()
            .filter(|song| song.artist == artist && song.album == album)
            .collect()
    }

    fn song(&self, artist: &str, album: &str, disc: u8, number: u8) -> Option<&Song> {
        self.iter().find(|song| {
            song.artist == artist
                && song.album == album
                && song.disc_number == disc
                && song.track_number == number
        })
    }

    fn search(&self, query: &str) -> Vec<Item> {
        let query = query.to_lowercase();

        let artists = self.artists();
        let albums = self.albums();

        let mut results: Vec<_> = artists
            .par_iter()
            .map(|artist| jaro(&query, Item::Artist(artist)))
            .chain(
                albums
                    .par_iter()
                    .map(|(artist, album)| jaro(&query, Item::Album((artist, album)))),
            )
            .chain(self.par_iter().map(|song| {
                jaro(
                    &query,
                    Item::Song((
                        song.artist.as_str(),
                        song.album.as_str(),
                        song.title.as_str(),
                        song.disc_number,
                        song.track_number,
                    )),
                )
            }))
            .flatten()
            .collect();

        if !query.is_empty() {
            //Sort results by score.
            results.par_sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        }

        if results.len() > 40 {
            //Remove the less accurate results.
            results.par_drain(40..);
        }

        results.par_sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
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
}

impl VDB for BTreeMap<&'static str, Vec<Album>> {
    fn artists(&self) -> Vec<&'static str> {
        let mut v: Vec<_> = self.keys().map(|key| *key).collect();
        v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
        v
    }

    fn albums(&self) -> Vec<(&'static str, &'static str)> {
        todo!()
    }

    fn albums_by_artist(&self, artist: &str) -> Vec<Vec<&Song>> {
        // match self.get(artist) {
        //     Some(albums) => Some(albums.as_slice()),
        //     None => None,
        // }
        todo!()
    }

    fn album(&self, artist: &str, album: &str) -> Vec<&Song> {
        if let Some(albums) = self.get(artist) {
            for al in albums {
                if album == al.title {
                    //TODO: This should just clone the references?
                    return al.songs.clone();
                }
            }
        }
        Vec::new()
    }

    fn song(&self, artist: &str, album: &str, disc: u8, number: u8) -> Option<&Song> {
        if let Some(albums) = self.get(artist) {
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

    fn search(&self, query: &str) -> Vec<Item> {
        let query = query.to_lowercase();

        let mut results = if query.is_empty() {
            let mut results = Vec::new();
            for (artist, albums) in self {
                for album in albums {
                    for song in &album.songs {
                        results.push((
                            Item::Song((
                                artist,
                                album.title,
                                &song.title,
                                song.disc_number,
                                song.track_number,
                            )),
                            1.0,
                        ))
                    }

                    results.push((Item::Album((artist, &album.title)), 1.0));
                }

                results.push((Item::Artist(artist), 1.0));
            }
            results
        } else {
            let results = RwLock::new(Vec::new());
            self.par_iter().for_each(|(artist, albums)| {
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
            RwLock::into_inner(results)
                .unwrap()
                .into_iter()
                .flatten()
                .collect()
        };

        if !query.is_empty() {
            //Sort results by score.
            results.par_sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        }

        if results.len() > 40 {
            //Remove the less accurate results.
            results.par_drain(40..);
        }

        results.par_sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
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
}

pub use btree::*;

mod btree {
    use super::*;

    pub type Database = BTreeMap<&'static str, Vec<Album>>;

    static mut SONGS: Vec<Song> = Vec::new();

    pub fn create() -> Result<Database, Box<dyn Error + Send + Sync>> {
        profile!();
        unsafe { SONGS = crate::db::read()? };
        let mut data: BTreeMap<&str, Vec<Album>> = BTreeMap::new();
        let mut albums: BTreeMap<(&str, &str), Vec<&Song>> = BTreeMap::new();

        // Add songs to albums.
        for song in unsafe { &SONGS } {
            match albums.entry((song.artist.as_str(), song.album.as_str())) {
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
    //TODO: Cleanup
    pub fn artists(db: &Database) -> Vec<&&str> {
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

    ///Search the database and return the 25 most accurate matches.
    pub fn search(db: &'static Database, query: &str) -> Vec<Item> {
        let query = query.to_lowercase();

        let mut results = if query.is_empty() {
            let mut results = Vec::new();
            for (artist, albums) in db {
                for album in albums {
                    for song in &album.songs {
                        results.push((
                            Item::Song((
                                artist,
                                album.title,
                                &song.title,
                                song.disc_number,
                                song.track_number,
                            )),
                            1.0,
                        ))
                    }

                    results.push((Item::Album((artist, &album.title)), 1.0));
                }

                results.push((Item::Artist(artist), 1.0));
            }
            results
        } else {
            let results = RwLock::new(Vec::new());
            db.par_iter().for_each(|(artist, albums)| {
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
            RwLock::into_inner(results)
                .unwrap()
                .into_iter()
                .flatten()
                .collect()
        };

        if !query.is_empty() {
            //Sort results by score.
            results.par_sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        }

        if results.len() > 40 {
            //Remove the less accurate results.
            results.par_drain(40..);
        }

        results.par_sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vector() {}
}

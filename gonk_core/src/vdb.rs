//! Virtual database
//!
//! Songs are taken from the physical database and stored in a `BTreeMap`
//!
//! Also contains code for querying artists, albums and songs.
//!
use crate::db::{Album, Song};
use crate::{database_path, strsim, Deserialize};
use std::collections::BTreeMap;
use std::{cmp::Ordering, fs, str::from_utf8_unchecked};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db() {
        let db = Database::new();
        dbg!(db.artists());
        dbg!(db.search("test"));
    }
}

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

///https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance
fn jaro(query: &str, input: Item) -> Result<(Item, f64), (Item, f64)> {
    let str = match input {
        Item::Artist(ref artist) => artist,
        Item::Album((_, ref album)) => album,
        Item::Song((_, _, ref song, _, _)) => song,
    };
    let acc = strsim::jaro_winkler(query, &str.to_lowercase());
    if acc > MIN_ACCURACY {
        Ok((input, acc))
    } else {
        Err((input, acc))
    }
}

//I feel like Box<[String, Box<Album>]> might have been a better choice.
pub struct Database {
    btree: BTreeMap<String, Vec<Album>>,
    pub len: usize,
}

impl Database {
    ///Read the database from disk and load it into memory.
    pub fn new() -> Self {
        let bytes = match fs::read(database_path()) {
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

        let query = query.to_lowercase();
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

        //Sort results by score.
        results.sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        if results.len() > MAX {
            //Remove the less accurate results.
            unsafe {
                results.set_len(MAX);
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
}

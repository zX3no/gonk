//! Virtual database
//!
//! Songs are taken from the physical database and stored in a `BTreeMap`
//!
//! Also contains code for querying artists, albums and songs.
//!
use crate::db::{Artwork, Song};
use crate::{Deserialize, strsim};
use std::ops::Range;
use std::{cmp::Ordering, fs, path::Path, str::from_utf8_unchecked};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    ///(Artist, Album, Name, Disc Number, Track Number)
    Song((String, String, String, u8, u8)),
    ///(Artist, Album)
    Album((String, String)),
    ///(Artist)
    Artist(String),
}

fn item_rank(item: &Item) -> u8 {
    match item {
        Item::Artist(_) => 0,
        Item::Album(_) => 1,
        Item::Song(_) => 2,
    }
}

// fn normalize_for_search(s: &str) -> String {
// use unicode_normalization::UnicodeNormalization;
// use unicode_normalization::char::is_combining_mark;
//     s.nfd()
//         .filter(|c| !is_combining_mark(*c))
//         .flat_map(char::to_lowercase)
//         .collect()
// }

const MIN_ACCURACY: f64 = 0.7;

pub fn jaro_score(query: &str, raw_text: &str) -> Option<f64> {
    // let text = normalize_for_search(raw_text);
    let text = raw_text;

    if query.is_empty() || text.is_empty() {
        return None;
    }

    if text == query {
        return Some(1.0);
    }

    if text.starts_with(query) {
        let coverage = query.len() as f64 / text.len() as f64;
        return Some(0.95 + 0.05 * coverage);
    }

    let mut best_fuzzy = strsim::jaro_winkler(query, &text);
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        if word == query {
            return Some(0.94);
        }

        if word.starts_with(query) {
            let coverage = query.len() as f64 / word.len() as f64;
            return Some(0.90 + 0.04 * coverage);
        }

        best_fuzzy = best_fuzzy.max(strsim::jaro_winkler(query, word));
    }

    if text.contains(query) {
        best_fuzzy = best_fuzzy.max(0.88);
    }

    if best_fuzzy > MIN_ACCURACY {
        Some(best_fuzzy)
    } else {
        None
    }
}

#[derive(Debug)]
pub struct ArtistEntry {
    pub name: String,
    pub albums: Range<usize>,
}

#[derive(Debug)]
pub struct AlbumEntry {
    pub title: String,
    pub songs: Range<usize>,
}

#[derive(Debug)]
pub struct Database {
    pub songs: Vec<Song>,
    pub albums: Vec<AlbumEntry>,
    pub artists: Vec<ArtistEntry>,
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

        let mut songs: Vec<Song> = unsafe { from_utf8_unchecked(&bytes) }
            .lines()
            .flat_map(Song::deserialize)
            .collect();

        songs.sort_unstable_by(|a, b| {
            a.artist
                .cmp(&b.artist)
                .then_with(|| a.album.cmp(&b.album))
                .then_with(|| a.title.cmp(&b.title))
        });

        let mut albums = Vec::new();
        let mut artists = Vec::new();

        let mut song_cursor = 0;
        let mut album_cursor = 0;

        for artist_songs in songs.chunk_by(|a, b| a.artist == b.artist) {
            let artist_first_album = album_cursor;

            for album_songs in artist_songs.chunk_by(|a, b| a.album == b.album) {
                song_cursor += album_songs.len();
                albums.push(AlbumEntry {
                    title: album_songs[0].album.clone(),
                    songs: song_cursor..song_cursor + album_songs.len(),
                });
                album_cursor += 1;
            }

            artists.push(ArtistEntry {
                name: artist_songs[0].artist.clone(),
                albums: artist_first_album..album_cursor,
            });
        }

        Self {
            songs,
            artists,
            albums,
        }
    }

    pub fn get_artists(&self) -> Vec<String> {
        self.artists.iter().map(|a| a.name.clone()).collect()
    }

    pub fn get_album_songs(&self, artist: &str, album: &str) -> Option<&[Song]> {
        let artist_idx = self
            .artists
            .binary_search_by_key(&artist, |a| a.name.as_str())
            .ok()?;
        let artist_albums = &self.albums[self.artists[artist_idx].albums.clone()];
        let album_idx = artist_albums
            .binary_search_by_key(&album, |alb| alb.title.as_str())
            .ok()?;
        Some(&self.songs[artist_albums[album_idx].songs.clone()])
    }

    pub fn get_artist_songs(&self, artist_name: &str) -> Option<&[Song]> {
        let artist_idx = self
            .artists
            .binary_search_by_key(&artist_name, |a| a.name.as_str())
            .ok()?;

        let artist = &self.artists[artist_idx];
        if artist.albums.is_empty() {
            return Some(&[]);
        }

        let first_album = &self.albums[artist.albums.start];
        let last_album = &self.albums[artist.albums.end - 1];

        Some(&self.songs[first_album.songs.start..last_album.songs.end])
    }

    pub fn get_artist_albums(&self, artist: &str) -> Option<&[AlbumEntry]> {
        let artist_idx = self
            .artists
            .binary_search_by_key(&artist, |a| a.name.as_str())
            .ok()?;
        Some(&self.albums[self.artists[artist_idx].albums.clone()])
    }

    ///Search the database and return the 25 most accurate matches.
    pub fn search(&self, query: &str) -> Vec<Item> {
        const MAX: usize = 40;
        // let query = normalize_for_search(query);

        if query.is_empty() {
            return self
                .songs
                .iter()
                .take(MAX)
                .map(|song| {
                    Item::Song((
                        song.artist.clone(),
                        song.album.clone(),
                        song.title.clone(),
                        song.disc_number,
                        song.track_number,
                    ))
                })
                .collect();
        }

        let mut results: Vec<(Item, f64)> = Vec::new();

        for artist in &self.artists {
            if let Some(score) = jaro_score(&query, &artist.name) {
                results.push((Item::Artist(artist.name.clone()), score));
            }

            for album in &self.albums[artist.albums.clone()] {
                if let Some(score) = jaro_score(&query, &album.title) {
                    results.push((
                        Item::Album((artist.name.clone(), album.title.clone())),
                        score,
                    ));
                }

                for song in &self.songs[album.songs.clone()] {
                    if let Some(score) = jaro_score(&query, &song.title) {
                        results.push((
                            Item::Song((
                                song.artist.clone(),
                                song.album.clone(),
                                song.title.clone(),
                                song.disc_number,
                                song.track_number,
                            )),
                            score,
                        ));
                    }
                }
            }
        }

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

    pub fn artwork(&self, song: &Song) -> Option<(&[u32], usize, usize)> {
        let songs = self.get_album_songs(&song.artist, &song.album)?;
        let first = songs.first()?;

        match first.artwork.as_ref()? {
            Artwork::Decoded(pixels, width, height) => Some((pixels, *width, *height)),
            Artwork::Compressed(_) => None,
        }
    }
}

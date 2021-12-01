use std::path::PathBuf;

use gronk_database::Database;
use gronk_types::Song;
use indicium::simple::{SearchIndex, SearchIndexBuilder, SearchType};

use crate::{index::Index, modes::SearchMode};
pub struct Search {
    pub query: String,
    pub prev_query: String,
    search_index: SearchIndex<usize>,
    pub mode: SearchMode,
    pub index: Index,
}
impl Search {
    //TODO: get albums and artists and insert them too
    pub fn new(songs: &[(usize, Song)], artists: &[String], albums: &[(String, String)]) -> Self {
        let mut search_index: SearchIndex<usize> = SearchIndexBuilder::default()
            .case_sensitive(&false)
            .search_type(&SearchType::Live)
            .build();

        for (index, elem) in songs {
            search_index.insert(index, elem);
        }

        Self {
            query: String::new(),
            prev_query: String::new(),
            search_index,
            mode: SearchMode::Search,
            index: Index::new(),
        }
    }
    pub fn on_key(&mut self, c: char) {
        // if let SearchMode::Search = &self.mode {
        //     self.prev_query = self.query.clone();
        //     self.query.push(c);
        // } else {
        //     match c {
        //         'k' => self.up(),
        //         'j' => self.down(),
        //         _ => (),
        //     }
        // }
    }
    pub fn up(&mut self) {
        // let len = self.results.len();
        // self.index.up(len);
    }
    pub fn down(&mut self) {
        // let len = self.results.len();
        // self.index.down(len);
    }
    pub fn search<T>(&mut self, query: &str, database: Database) {
        self.query = query.to_string();
        let songs: Vec<usize> = self
            .search_index
            .search(query)
            .iter()
            .map(|i| **i)
            .collect();

        let songs = database.get_songs_from_ids(&songs);

        let artists: Vec<String> = self
            .artists
            .iter()
            .filter_map(|artist| {
                if artist.contains(query) {
                    Some(artist.clone())
                } else {
                    None
                }
            })
            .collect();

        let albums: Vec<(String, String)> = self
            .albums
            .iter()
            .filter_map(|(album, artist)| {
                if artist.contains(query) || album.contains(query) {
                    Some((album.clone(), artist.clone()))
                } else {
                    None
                }
            })
            .collect();

        let l = List {
            songs,
            albums,
            artists,
            selection: None,
            results: Vec::new(),
        };

        dbg!(l.get());
    }
    // pub fn changed(&mut self) -> bool {
    //     if self.query != self.prev_query {
    //         self.prev_query = self.query.clone();
    //         true
    //     } else {
    //         false
    //     }
    // }
    pub fn exit(&mut self) {
        self.mode.next();
        self.index.select(None);
    }
    pub fn state(&self) -> Option<usize> {
        self.index.index
    }
    pub fn get_selected(&self) -> Option<usize> {
        self.index.index
    }
    pub fn reset(&mut self) {
        self.mode.reset();
        self.index.select(None);
    }
    pub fn is_empty(&self) -> bool {
        // self.results.is_empty() && self.query.is_empty()
        true
    }
}

#[derive(Debug)]
struct List {
    songs: Vec<Song>,
    albums: Vec<(String, String)>,
    artists: Vec<String>,
    selection: Option<usize>,
    //store get in this
    results: Vec<Song>,
}

impl List {
    // pub fn filter(&self, query: &str) {}
    pub fn get(&self) -> Vec<Song> {
        let mut out: Vec<Song> = self
            .artists
            .iter()
            .map(|artist| Song {
                number: 0,
                name: String::new(),
                disc: 0,
                artist: artist.clone(),
                album: String::new(),
                path: PathBuf::new(),
            })
            .collect();

        let mut albums: Vec<Song> = self
            .albums
            .iter()
            .map(|(album, artist)| Song {
                number: 0,
                name: String::new(),
                disc: 0,
                artist: artist.clone(),
                album: album.clone(),
                path: PathBuf::new(),
            })
            .collect();
        out.append(&mut albums);
        out.append(&mut self.songs.clone());

        out
    }
}
//My ideal search algorithm

//"Burger", "b23u89r6g88e987r", "borger", "bugs", "test"

//Query: burger

//1: "burger"
//2: "borger"
//3: "bugs"
//4: "b23u89r6g88e987r"

//the most important part is the distance of the characters
//not how many of them match

//"Sketch 1", "s23c0et23"

//query: scet

//1: "sketch 1"
//2: "s23c0et23"

//Gronk Search

//Data: Artist, Album and Song names

//query: Badbadnotgood

//Results:
//Badbadnotgood
//IV
//Talk Memory
//1. And That, Too

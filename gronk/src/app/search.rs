use gronk_types::Song;
use indicium::simple::{SearchIndex, SearchIndexBuilder, SearchType};

use crate::{index::Index, modes::SearchMode};

pub struct Search {
    pub query: String,
    pub prev_query: String,
    pub search_index: SearchIndex<usize>,
    pub results: Vec<usize>,
    pub mode: SearchMode,
    pub index: Index,
}

impl Search {
    pub fn new(songs: &[(usize, Song)]) -> Self {
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
            results: Vec::new(),
            mode: SearchMode::Search,
            index: Index::new(),
        }
    }
    pub fn on_key(&mut self, c: char) {
        if let SearchMode::Search = &self.mode {
            self.prev_query = self.query.clone();
            self.query.push(c);
        } else {
            match c {
                'k' => self.up(),
                'j' => self.down(),
                _ => (),
            }
        }
    }
    pub fn up(&mut self) {
        let len = self.results.len();
        self.index.up(len);
    }
    pub fn down(&mut self) {
        let len = self.results.len();
        self.index.down(len);
    }
    pub fn update_search(&mut self) {
        self.results = self
            .search_index
            .search(&self.query)
            .iter()
            .map(|i| **i)
            .collect();
    }
    pub fn changed(&mut self) -> bool {
        if self.query != self.prev_query {
            self.prev_query = self.query.clone();
            true
        } else {
            false
        }
    }
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

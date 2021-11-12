use gronk_types::Song;
use indicium::simple::{SearchIndex, SearchIndexBuilder, SearchType};

pub struct Search {
    pub query: String,
    pub prev_query: String,
    pub results: Vec<usize>,
}

impl Search {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            prev_query: String::new(),
            results: Vec::new(),
        }
    }
    pub fn push(&mut self, c: char) {
        self.prev_query = self.query.clone();
        self.query.push(c);
    }
    pub fn get_song_ids(&mut self, songs: &Vec<(usize, Song)>) {
        let mut search_index: SearchIndex<usize> = SearchIndexBuilder::default()
            .case_sensitive(&false)
            .search_type(&SearchType::Live)
            .build();

        for (index, elem) in songs {
            search_index.insert(&index, elem);
        }

        self.results = search_index
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

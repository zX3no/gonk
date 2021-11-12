use indicium::simple::{Indexable, SearchIndex};
use simsearch::{SearchOptions, SimSearch};

pub struct Search {
    pub query: String,
    pub prev_query: String,
    pub ids: Vec<(usize, String)>,
    pub results: Vec<usize>,
}

impl Search {
    pub fn new(ids: Vec<(usize, String)>) -> Self {
        Self {
            query: String::new(),
            prev_query: String::new(),
            ids,
            results: Vec::new(),
        }
    }
    pub fn push(&mut self, c: char) {
        self.prev_query = self.query.clone();
        self.query.push(c);
    }
    pub fn get_song_ids(&mut self) {
        let options = SearchOptions::new().case_sensitive(false);
        let mut engine: SimSearch<usize> = SimSearch::new_with(options);

        for (id, name) in &self.ids {
            engine.insert(*id, &name);
        }

        self.results = engine.search(self.query.as_str());
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

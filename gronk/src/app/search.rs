use simsearch::{SearchOptions, SimSearch};

pub struct Search {
    pub query: String,
    pub ids: Vec<(usize, String)>,
}

impl Search {
    pub fn new(ids: Vec<(usize, String)>) -> Self {
        Self {
            query: String::new(),
            ids,
        }
    }
    pub fn get_song_ids(&self) -> Vec<usize> {
        let options = SearchOptions::new().case_sensitive(false);
        let mut engine: SimSearch<usize> = SimSearch::new_with(options);

        for (id, name) in &self.ids {
            engine.insert(*id, &name);
        }

        engine.search(self.query.as_str())
    }
}

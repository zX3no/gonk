use crossterm::event::KeyModifiers;
use gronk_search::{SearchEngine, SearchItem};
use gronk_types::Song;

use crate::index::Index;
use crate::modes::SearchMode;

pub struct Search {
    engine: SearchEngine,
    query: String,
    prev_query: String,
    pub results: Vec<SearchItem>,
    pub mode: SearchMode,
    pub index: Index,
}
impl Search {
    pub fn new(songs: &[(Song, usize)], albums: &[(String, String)], artists: &[String]) -> Self {
        let mut engine = SearchEngine::new();

        let songs: Vec<_> = songs
            .iter()
            .map(|(song, id)| SearchItem::song(&song.name, *id))
            .collect();

        let albums: Vec<_> = albums
            .iter()
            .map(|(name, artist)| SearchItem::album(name, artist))
            .collect();

        let artists: Vec<_> = artists
            .iter()
            .map(|name| SearchItem::artist(&name))
            .collect();

        engine.insert_vec(songs);
        engine.insert_vec(albums);
        engine.insert_vec(artists);

        Self {
            engine,
            query: String::new(),
            prev_query: String::new(),
            results: Vec::new(),
            mode: SearchMode::Search,
            index: Index::new(),
        }
    }
    pub fn update_search(&mut self) {
        self.results = self.engine.search(&self.query);
        // dbg!(&self.query);
        // dbg!(&self.results);
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
    pub fn exit(&mut self) {
        self.mode.next();
        self.index.select(None);
    }
    pub fn state(&self) -> Option<usize> {
        self.index.index
    }
    pub fn get_selected(&self) -> Option<&SearchItem> {
        if let Some(index) = self.index.selected() {
            self.results.get(index)
        } else {
            None
        }
    }
    pub fn reset(&mut self) {
        self.mode.reset();
        self.index.select(None);
    }
    pub fn is_empty(&self) -> bool {
        self.results.is_empty() && self.query.is_empty()
    }
    pub fn on_backspace(&mut self, modifiers: KeyModifiers) {
        if modifiers == KeyModifiers::CONTROL {
            self.query = String::new();
        } else {
            self.query.pop();
        }
    }
    pub fn query_changed(&mut self) -> bool {
        if self.query != self.prev_query {
            self.prev_query = self.query.clone();
            true
        } else {
            false
        }
    }
    pub fn set_cursor(&self) -> bool {
        self.index.is_none() && self.query.is_empty()
    }
    pub fn query_len(&self) -> u16 {
        self.query.len() as u16
    }
    pub fn get_query(&self) -> String {
        self.query.clone()
    }
}

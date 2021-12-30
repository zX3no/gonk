use crossterm::event::KeyModifiers;
use gronk_database::Database;
use gronk_search::{ItemType, SearchEngine, SearchItem};
use gronk_types::Song;

use crate::index::Index;

pub enum SearchMode {
    Search,
    Select,
}
impl SearchMode {
    pub fn next(&mut self) {
        match self {
            SearchMode::Search => *self = SearchMode::Select,
            SearchMode::Select => *self = SearchMode::Search,
        }
    }

    pub fn reset(&mut self) {
        *self = SearchMode::Search;
    }
}

pub struct Search<'a> {
    db: &'a Database,
    engine: SearchEngine,
    query: String,
    prev_query: String,
    mode: SearchMode,
    results: Index<SearchItem>,
}

impl<'a> Search<'a> {
    pub fn new(
        db: &'a Database,
        songs: &[(Song, usize)],
        albums: &[(String, String)],
        artists: &[String],
    ) -> Self {
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
            .map(|name| SearchItem::artist(name))
            .collect();

        engine.insert_vec(songs);
        engine.insert_vec(albums);
        engine.insert_vec(artists);

        Self {
            db,
            engine,
            query: String::new(),
            prev_query: String::new(),
            results: Index::default(),
            mode: SearchMode::Search,
        }
    }
    //TODO: this function name is misleading
    //fix?
    pub fn get_songs(&mut self) -> Option<Vec<Song>> {
        if let SearchMode::Search = self.mode {
            if !self.is_empty() {
                self.mode.next();
                self.results.select(Some(0));
            }
            None
        } else if let Some(item) = self.results.selected() {
            match item.item_type {
                ItemType::Song => Some(vec![self.db.get_song_from_id(item.song_id.unwrap())]),
                ItemType::Album => Some(
                    self.db
                        .get_album(item.album_artist.as_ref().unwrap(), &item.name),
                ),
                ItemType::Artist => Some(self.db.get_artist(&item.name)),
            }
        } else {
            None
        }
    }
    pub fn update_search(&mut self) {
        self.results.data = self.engine.search(&self.query);
        // dbg!(&self.query);
        // dbg!(&self.results);
    }
    pub fn on_key(&mut self, c: char) {
        if let SearchMode::Search = &self.mode {
            self.prev_query = self.query.clone();
            self.query.push(c);
        } else {
            match c {
                'k' => self.results.up(),
                'j' => self.results.down(),
                _ => (),
            }
        }
    }
    pub fn exit(&mut self) {
        self.mode.next();
        self.results.select(None);
    }
    pub fn reset(&mut self) {
        self.mode.reset();
        self.results.select(None);
    }
    pub fn up(&mut self) {
        self.results.up();
    }
    pub fn down(&mut self) {
        self.results.down();
    }
    pub fn on_backspace(&mut self, modifiers: KeyModifiers) {
        match self.mode {
            SearchMode::Search => {
                if modifiers == KeyModifiers::CONTROL {
                    self.query = String::new();
                } else {
                    self.query.pop();
                }
            }
            SearchMode::Select => self.exit(),
        }
    }
    pub fn has_query_changed(&mut self) -> bool {
        if self.query != self.prev_query {
            self.prev_query = self.query.clone();
            true
        } else {
            false
        }
    }
    pub fn empty_cursor(&self) -> bool {
        self.results.is_none() && self.query.is_empty()
    }
    pub fn show_cursor(&self) -> bool {
        match self.mode {
            SearchMode::Search => true,
            SearchMode::Select => false,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.results.is_empty() && self.query.is_empty()
    }
    pub fn query_len(&self) -> u16 {
        self.query.len() as u16
    }
    pub fn get_query(&self) -> String {
        self.query.clone()
    }
    pub fn results(&self) -> &Vec<SearchItem> {
        &self.results.data
    }
    pub fn selected(&self) -> Option<usize> {
        self.results.index()
    }
}

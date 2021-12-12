use std::fmt::Display;

pub struct SearchEngine {
    data: Vec<SearchItem>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
    pub fn insert(&mut self, item: SearchItem) {
        self.data.push(item);
    }
    pub fn insert_vec(&mut self, items: Vec<SearchItem>) {
        self.data.extend(items);
    }
    pub fn search(&self, query: &str) -> Vec<SearchItem> {
        let mut results = Vec::new();
        for item in &self.data {
            let acc = strsim::jaro_winkler(&query.to_lowercase(), &item.name.to_lowercase());

            if acc > 0.75 {
                results.push((item, acc))
            }
        }
        results.sort_by(|(_, a), (_, b)| b.partial_cmp(&a).unwrap());
        results.into_iter().map(|(item, _)| item.clone()).collect()
    }
}

#[derive(Clone, Debug)]
pub enum ItemType {
    Song,
    Album,
    Artist,
}

#[derive(Clone, Debug)]
pub struct SearchItem {
    pub name: String,
    pub song_id: Option<usize>,
    pub album_artist: Option<String>,
    pub item_type: ItemType,
}

impl SearchItem {
    pub fn song(title: &str, id: usize) -> Self {
        Self {
            name: title.to_string(),
            song_id: Some(id),
            album_artist: None,
            item_type: ItemType::Song,
        }
    }
    pub fn album(name: &str, artist: &str) -> Self {
        Self {
            name: name.to_string(),
            song_id: None,
            album_artist: Some(artist.to_string()),
            item_type: ItemType::Album,
        }
    }
    pub fn artist(name: &str) -> Self {
        Self {
            name: name.to_string(),
            song_id: None,
            album_artist: None,
            item_type: ItemType::Artist,
        }
    }
}

impl Display for SearchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

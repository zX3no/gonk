use std::fmt::Display;

#[derive(Default)]
pub struct SearchEngine {
    data: Vec<SearchItem>,
}

impl SearchEngine {
    pub fn insert(&mut self, item: SearchItem) {
        self.data.push(item);
    }
    pub fn insert_vec(&mut self, items: Vec<SearchItem>) {
        self.data.extend(items);
    }
    pub fn search(&self, query: &str) -> Vec<SearchItem> {
        let mut results = Vec::new();
        for item in &self.data {
            let acc = match item.item_type {
                ItemType::Song => strsim::jaro_winkler(
                    &query.to_lowercase(),
                    &item.song.as_ref().unwrap().to_lowercase(),
                ),
                ItemType::Album => strsim::jaro_winkler(
                    &query.to_lowercase(),
                    &item.album.as_ref().unwrap().to_lowercase(),
                ),
                ItemType::Artist => strsim::jaro_winkler(
                    &query.to_lowercase(),
                    &item.artist.as_ref().unwrap().to_lowercase(),
                ),
            };

            if acc > 0.75 {
                results.push((item, acc))
            }
        }
        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        results.into_iter().map(|(item, _)| item.clone()).collect()
    }
}

#[derive(Clone, Debug)]
pub enum ItemType {
    Song,
    Album,
    Artist,
}

//TODO: change this to use generics
#[derive(Clone, Debug)]
pub struct SearchItem {
    pub song_id: Option<usize>,
    pub song: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub item_type: ItemType,
}

impl SearchItem {
    pub fn song(song: &str, id: usize, album: &str, artist: &str) -> Self {
        Self {
            song: Some(song.to_string()),
            song_id: Some(id),
            artist: Some(artist.to_string()),
            album: Some(album.to_string()),
            item_type: ItemType::Song,
        }
    }
    pub fn album(name: &str, artist: &str) -> Self {
        Self {
            song: None,
            song_id: None,
            album: Some(name.to_string()),
            artist: Some(artist.to_string()),
            item_type: ItemType::Album,
        }
    }
    pub fn artist(name: &str) -> Self {
        Self {
            song: None,
            song_id: None,
            album: None,
            artist: Some(name.to_string()),
            item_type: ItemType::Artist,
        }
    }
}

impl Display for SearchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.item_type {
            ItemType::Song => write!(f, "{}", self.song.as_ref().unwrap()),
            ItemType::Album => write!(f, "{}", self.album.as_ref().unwrap()),
            ItemType::Artist => write!(f, "{}", self.artist.as_ref().unwrap()),
        }
    }
}

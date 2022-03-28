pub trait SearchItem {
    fn song(&self) -> Option<(usize, &String)>;
    fn album(&self) -> Option<&String>;
    fn artist(&self) -> Option<&String>;
}

#[derive(Clone)]
pub enum Item {
    Song(Song),
    Album(Album),
    Artist(Artist),
}

impl SearchItem for Item {
    fn song(&self) -> Option<(usize, &String)> {
        match self {
            Item::Song(item) => item.song(),
            Item::Album(item) => item.song(),
            Item::Artist(item) => item.song(),
        }
    }

    fn album(&self) -> Option<&String> {
        match self {
            Item::Song(item) => item.album(),
            Item::Album(item) => item.album(),
            Item::Artist(item) => item.album(),
        }
    }

    fn artist(&self) -> Option<&String> {
        match self {
            Item::Song(item) => item.artist(),
            Item::Album(item) => item.artist(),
            Item::Artist(item) => item.artist(),
        }
    }
}

pub struct Engine<T: SearchItem> {
    pub data: Vec<T>,
}

impl<T: SearchItem> Engine<T> {
    pub fn push(&mut self, item: T) {
        self.data.push(item);
    }
}

impl<T: SearchItem> Default for Engine<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct Song {
    id: usize,
    name: String,
    album: String,
    artist: String,
}

impl Song {
    pub fn new(id: usize, name: String, album: String, artist: String) -> Self {
        Self {
            id,
            name,
            album,
            artist,
        }
    }
}

impl SearchItem for Song {
    fn song(&self) -> Option<(usize, &String)> {
        Some((self.id, &self.name))
    }

    fn artist(&self) -> Option<&String> {
        Some(&self.artist)
    }

    fn album(&self) -> Option<&String> {
        Some(&self.album)
    }
}

#[derive(Clone)]
pub struct Album {
    name: String,
    artist: String,
}
impl Album {
    pub fn new(name: String, artist: String) -> Self {
        Self { name, artist }
    }
}

impl SearchItem for Album {
    fn song(&self) -> Option<(usize, &String)> {
        None
    }

    fn album(&self) -> Option<&String> {
        Some(&self.name)
    }

    fn artist(&self) -> Option<&String> {
        Some(&self.artist)
    }
}

#[derive(Clone)]
pub struct Artist {
    name: String,
}

impl Artist {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl SearchItem for Artist {
    fn song(&self) -> Option<(usize, &String)> {
        None
    }

    fn album(&self) -> Option<&String> {
        None
    }

    fn artist(&self) -> Option<&String> {
        Some(&self.name)
    }
}

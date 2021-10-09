use std::cmp::Ordering;

use crate::database::song::Song;

pub struct Album {
    title: String,
    songs: Vec<Song>,
}

impl Ord for Album {
    fn cmp(&self, other: &Self) -> Ordering {
        self.title.cmp(&other.title)
        // (self.name, &self.albums).cmp(&(other.name, &other.albums))
    }
}
impl PartialOrd for Album {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Album {
    fn eq(&self, other: &Self) -> bool {
        self.title == self.title
        // (self.name, &self.albums) == (&(other.name, &other.albums))
    }
}

impl Eq for Album {}

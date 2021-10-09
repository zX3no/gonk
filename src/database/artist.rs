use std::cmp::Ordering;

use crate::database::album::Album;
pub struct Artist {
    name: String,
    albums: Vec<Album>,
}
impl Ord for Artist {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
        // (self.name, &self.albums).cmp(&(other.name, &other.albums))
    }
}
impl PartialOrd for Artist {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Artist {
    fn eq(&self, other: &Self) -> bool {
        self.name == self.name
        // (self.name, &self.albums) == (&(other.name, &other.albums))
    }
}

impl Eq for Artist {}

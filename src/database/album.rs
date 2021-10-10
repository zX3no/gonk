use crate::database::song::Song;

#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub songs: Vec<Song>,
}
impl Album {
    pub fn at(&self, track_number: usize) -> Option<&Song> {
        self.songs.get(track_number)
    }
}

use crate::database::song::Song;

#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub songs: Vec<Song>,
}
impl Album {
    pub fn track(&self, track_number: u16) -> Option<&Song> {
        for song in &self.songs {
            if song.number == track_number {
                return Some(song);
            }
        }
        None
    }
}

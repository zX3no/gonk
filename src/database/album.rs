use crate::database::song::Song;

#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub songs: Vec<Song>,
}

use crate::database::song::Song;

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

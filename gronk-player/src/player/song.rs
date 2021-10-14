use std::path::PathBuf;

pub struct Song {
    title: String,
    album: String,
    artist: String,
    path: PathBuf,
    track_number: u32,
    duration: f32,
}
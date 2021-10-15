use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Song {
    title: String,
    album: String,
    artist: String,
    path: PathBuf,
    track_number: u32,
    duration: f64,
    elapsed: Option<f64>,
}

impl Song {
    pub fn from(path: &str) -> Self {
        Self {
            title: String::new(),
            album: String::new(),
            artist: String::new(),
            path: PathBuf::from(path),
            track_number: 1,
            duration: 0.0,
            elapsed: None,
        }
    }
    pub fn get_duration(&self) -> f64 {
        self.duration
    }
    pub fn get_elapsed(&self) -> Option<f64> {
        self.elapsed
    }
    pub fn update_elapsed(&mut self, elapsed: f64) {
        self.elapsed = Some(elapsed);
    }
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
}
impl PartialEq for Song {
    fn eq(&self, other: &Song) -> bool {
        self.title == other.title && self.path == other.path
    }
}

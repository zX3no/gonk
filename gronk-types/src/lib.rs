#[derive(Debug, Clone)]
pub struct Song {
    pub number: u16,
    pub disc: u16,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub path: std::path::PathBuf,
}
impl Song {
    pub fn from(path: &str) -> Self {
        if let Ok(tag) = audiotags::Tag::new().read_from_path(&path) {
            let artist = if let Some(artist) = tag.album_artist() {
                artist.to_string()
            } else if let Some(artist) = tag.artist() {
                artist.to_string()
            } else {
                panic!("no artist for {:?}", path);
            };
            let disc = tag.disc_number().unwrap_or(1);

            return Self {
                number: tag.track_number().unwrap(),
                disc,
                name: tag.title().unwrap().to_string(),
                album: tag.album_title().unwrap().to_string(),
                artist,
                path: std::path::PathBuf::from(path),
            };
        }
        panic!();
    }
}
impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
            && self.disc == other.disc
            && self.name == other.name
            && self.album == other.album
            && self.artist == other.artist
            && self.path == other.path
    }
}
impl Default for Song {
    fn default() -> Self {
        Self {
            number: Default::default(),
            disc: Default::default(),
            name: Default::default(),
            album: Default::default(),
            artist: Default::default(),
            path: Default::default(),
        }
    }
}

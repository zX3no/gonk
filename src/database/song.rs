use std::path::PathBuf;

use audiotags::Tag;
#[derive(Debug)]
pub struct Song {
    pub title: String,
    pub number: u16,
    pub album: String,
    pub album_artist: String,
    pub duration: u32,
    pub year: i32,
}

impl Song {
    pub fn from(path: PathBuf) -> Self {
        let tag = Tag::new().read_from_path(&path).unwrap();
        let exit = path.to_str().unwrap();

        //this is dank
        let album_artist = if let Some(artist) = tag.album_artist() {
            artist.to_string()
        } else if let Some(artist) = tag.artist() {
            artist.to_string()
        } else {
            panic!("{}", exit);
        };

        Song {
            title: tag.title().unwrap().to_string(),
            number: tag.track_number().unwrap(),
            album: tag.album_title().unwrap().to_string(),
            album_artist,
            //todo
            duration: 0,
            year: tag.year().unwrap(),
        }
    }
}

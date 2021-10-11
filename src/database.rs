use audiotags::Tag;
use std::path::PathBuf;
///
///
///
#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}

impl Artist {
    pub fn album(&self, name: &str) -> Option<&Album> {
        let mut out = None;
        for album in &self.albums {
            if album.title == name {
                out = Some(album);
            }
        }
        return out;
    }
}
///
///
///
#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub songs: Vec<Song>,
}
impl Album {
    pub fn track(&self, track_number: &u16) -> Option<&Song> {
        for song in &self.songs {
            if &song.number == track_number {
                return Some(song);
            }
        }
        None
    }
}
///
///
///
#[derive(Debug, Clone)]
pub struct Song {
    pub title: String,
    pub number: u16,
    pub album: String,
    pub album_artist: String,
    pub duration: u32,
    pub year: i32,
    pub path: PathBuf,
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
            path,
        }
    }
}

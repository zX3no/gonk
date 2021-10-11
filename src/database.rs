use audiotags::Tag;
use hashbrown::HashMap;
use itertools::Itertools;
use jwalk::WalkDir;
use std::path::{Path, PathBuf};
pub struct Database {
    pub data: HashMap<String, Artist>,
}
impl Database {
    pub fn create() -> Self {
        let path = Path::new(r"D:\OneDrive\Music");

        let mut songs: HashMap<String, Song> = HashMap::new();
        for entry in WalkDir::new(path) {
            if let Ok(e) = entry {
                if let Some(ex) = e.path().extension() {
                    if ex == "flac" {
                        let song = Song::from(e.path());

                        songs.insert(song.title.clone(), song);
                    }
                }
            }
        }

        let mut albums: HashMap<String, Album> = HashMap::new();

        for (_, v) in &songs {
            if albums.get(&v.album).is_some() {
                albums.get_mut(&v.album).unwrap().songs.push(v.clone());
            } else {
                albums.insert(
                    v.album.to_string(),
                    Album {
                        title: v.album.clone(),
                        artist: v.album_artist.clone(),
                        songs: vec![v.clone()],
                        total_discs: v.total_disc,
                    },
                );
            }
        }

        let mut artists: HashMap<String, Artist> = HashMap::new();

        for album in &albums {
            let (_, v) = album;

            if artists.get(&v.artist).is_some() {
                artists.get_mut(&v.artist).unwrap().albums.push(v.clone());
            } else {
                artists.insert(
                    v.artist.clone(),
                    Artist {
                        name: v.artist.clone(),
                        albums: vec![v.clone()],
                    },
                );
            }
        }

        Self { data: artists }
    }
    pub fn albums(&self, name: &String) -> Vec<String> {
        self.data[name]
            .albums
            .iter()
            .sorted_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .map(|a| a.title.clone())
            .collect()
    }
    pub fn tracks(&self, artist: &String, album: &String) -> Vec<String> {
        //this will cause a crash
        self.data[artist]
            .album(album)
            .unwrap()
            .songs
            .iter()
            .sorted_by(|a, b| a.disc.cmp(&b.disc).then(a.number.cmp(&b.number)))
            .map(|song| {
                let mut out = song.number.to_string();
                out.push_str(". ");
                out.push_str(&song.title);
                out.clone()
            })
            .collect()
    }
    pub fn path(&self, artist: &String, album: &String, track: &u16) -> PathBuf {
        self.data[artist]
            .album(album)
            .unwrap()
            .track(track)
            .unwrap()
            .path
            .clone()
    }
}
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
    pub total_discs: u16,
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
    pub disc: u16,
    pub total_disc: u16,
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

        let total_disc = tag.total_discs().unwrap_or(1);
        let disc = tag.disc_number().unwrap_or(1);

        Song {
            title: tag.title().unwrap().to_string(),
            number: tag.track_number().unwrap(),
            album: tag.album_title().unwrap().to_string(),
            album_artist,
            //todo
            duration: 0,
            year: tag.year().unwrap(),
            path,
            disc,
            total_disc,
        }
    }
}

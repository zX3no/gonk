use audiotags::Tag;
use hashbrown::HashMap;
use jwalk::WalkDir;
use std::path::{Path, PathBuf};
pub struct Database {
    artists: Vec<Artist>,
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

                        songs.insert(song.name.clone(), song);
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
                        name: v.album.clone(),
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

        let artists = artists.values().map(|v| v.clone()).collect();

        Self { artists }
    }
    pub fn get_albums_by_artist(&self, artist: &String) -> Vec<Album> {
        let a = self.get_artist(artist);
        a.albums
    }
    pub fn get_artist(&self, artist: &String) -> Artist {
        for a in &self.artists {
            if &a.name == artist {
                return a.clone();
            }
        }
        panic!();
    }
    pub fn get_artists(&self) -> Vec<Artist> {
        self.artists.clone()
    }
    pub fn get_album(&self, artist: &String, album: &String) -> Vec<Song> {
        let artist = self.get_artist(artist);
        for a in artist.albums {
            if &a.name == album {
                return a.songs;
            }
        }
        panic!();
    }
    pub fn get_song(&self, artist: &String, album: &String, track: &u16) -> Song {
        let a = self.get_album(artist, album);

        for song in a {
            if &song.track_number == track {
                return song;
            }
        }
        panic!();
    }
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}

impl Artist {
    pub fn album(&self, name: &str) -> Option<&Album> {
        let mut out = None;
        for album in &self.albums {
            if album.name == name {
                out = Some(album);
            }
        }
        return out;
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub songs: Vec<Song>,
    pub total_discs: u16,
}
impl Album {
    pub fn track(&self, track_number: &u16) -> Option<&Song> {
        for tracks in &self.songs {
            if &tracks.track_number == track_number {
                return Some(tracks);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Song {
    pub name: String,
    pub name_with_number: String,
    pub track_number: u16,
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

        let track_number = tag.track_number().unwrap();
        let name = tag.title().unwrap().to_string();

        let name_with_number = format!("{}. {}", track_number.to_string(), name);

        Song {
            name,
            name_with_number,
            track_number,
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

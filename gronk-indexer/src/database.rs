#![allow(dead_code)]
use audiotags::Tag;
use hashbrown::HashMap;
use jwalk::WalkDir;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    pub path: PathBuf,
    pub artists: Vec<Artist>,
}

impl Database {
    pub fn new(path: &str) -> Self {
        if let Some(database) = Database::read() {
            return database;
        }
        let path = PathBuf::from(path);
        let database = Database::create(&path);
        Database::write(&database);
        return database;
    }

    //~550ms
    pub fn read() -> Option<Self> {
        if let Ok(database) = fs::read_to_string("music.toml") {
            if database.is_empty() {
                return None;
            }
            return Some(toml::from_str(&database).unwrap());
        }
        None
    }

    //~60ms
    pub fn write(database: &Database) {
        //todo Database::path() -> C:\Users\Bay\Appdata\Gronk\music.toml
        let mut file = File::create("music.toml").unwrap();
        let output = toml::to_string(&database).unwrap();
        file.write_all(output.as_bytes()).unwrap();
    }

    pub fn update(&mut self) {
        *self = Database::create(&self.path);
        Database::write(self);
    }

    //~1.4s
    pub fn create(path: &PathBuf) -> Self {
        let mut songs: HashMap<String, Song> = HashMap::new();

        for entry in WalkDir::new(path).into_iter().flatten() {
            if let Some(ex) = entry.path().extension() {
                if ex == "flac" || ex == "mp3" || ex == "m4a" || ex == "wav" {
                    //this is slow
                    let song = Song::from(entry.path());
                    songs.insert(song.name.clone(), song.clone());
                }
            }
        }
        let mut albums: HashMap<String, Album> = HashMap::new();

        for (_, v) in songs {
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

        for album in albums {
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

        let artists: Vec<Artist> = artists.values().cloned().collect();

        Self {
            artists,
            path: path.clone(),
        }
    }

    //~20us
    pub fn get_albums(&self) -> Vec<&Album> {
        let mut albums = Vec::new();
        for artist in &self.artists {
            albums.extend(&artist.albums);
        }
        return albums;
    }

    //~160us
    pub fn get_songs(&self) -> Vec<&Song> {
        let albums = self.get_albums();
        let mut songs = Vec::new();
        for album in albums {
            songs.extend(&album.songs);
        }
        return songs;
    }

    //~240us
    pub fn find_song(&self, name: &str) -> Option<&Song> {
        for song in self.get_songs() {
            if song.name == name {
                return Some(song);
            }
        }
        None
    }

    //~30us
    pub fn find_album(&self, name: &str) -> Option<&Album> {
        for album in &self.get_albums() {
            if album.name == name {
                return Some(album);
            }
        }
        None
    }

    //~4us
    pub fn find_artist(&self, name: &str) -> Option<&Artist> {
        for artist in &self.artists {
            if artist.name == name {
                return Some(artist);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub total_discs: u16,
    pub songs: Vec<Song>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub name: String,
    pub name_with_number: String,
    pub track_number: u16,
    pub album: String,
    pub album_artist: String,
    pub year: i32,
    pub path: PathBuf,
    //todo track length / not implemented into audiotags
    pub disc: u16,
    pub total_disc: u16,
    pub duration: f64,
    pub elapsed: Option<f64>,
}

impl Song {
    pub fn from(path: PathBuf) -> Self {
        //this is slow
        let tag = Tag::new().read_from_path(&path).unwrap();

        let album_artist = if let Some(artist) = tag.album_artist() {
            artist.to_string()
        } else if let Some(artist) = tag.artist() {
            artist.to_string()
        } else {
            panic!("no artist for {:?}", path);
        };

        let total_disc = tag.total_discs().unwrap_or(1);
        let disc = tag.disc_number().unwrap_or(1);

        let track_number = tag.track_number().unwrap();
        let name = tag.title().unwrap().to_string();

        let name_with_number = format!("{}. {}", track_number.to_string(), name);

        let year = tag.year().unwrap_or(0);

        Song {
            name,
            name_with_number,
            track_number,
            album: tag.album_title().unwrap().to_string(),
            album_artist,
            year,
            path,
            disc,
            total_disc,
            duration: 0.0,
            elapsed: None,
        }
    }
    pub fn update_elapsed(&mut self, elapsed: f64) {
        self.elapsed = Some(elapsed);
    }
}
impl PartialEq for Song {
    fn eq(&self, other: &Song) -> bool {
        self.name == other.name && self.path == other.path
    }
}

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
    pub selected_artist: Artist,
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
    pub fn create(path: &Path) -> Self {
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
                        selected_song: v.clone(),
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
                        selected_album: v.clone(),
                    },
                );
            }
        }

        let artists: Vec<Artist> = artists.values().cloned().collect();

        //todo: fix?
        let selected_artist = artists.clone().first().unwrap().clone();

        Self {
            artists,
            path: path.to_path_buf(),
            selected_artist,
        }
    }

    pub fn artists_down(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_artist() {
            //try to move up
            if let Some(artist) = self.artists.get(i + 1) {
                //if we can update the selected artist
                self.selected_artist = artist.clone();
            } else {
                if let Some(artist) = self.artists.first() {
                    //if we can't reset to first artist
                    self.selected_artist = artist.clone();
                } else {
                    panic!("first returned nothing");
                }
            }
        } else {
            panic!("no selected artist?");
        }
    }
    pub fn artists_up(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_artist() {
            //try to move up
            if i == 0 {
                if let Some(artist) = self.artists.last() {
                    //if we can't reset to first artist
                    self.selected_artist = artist.clone();
                } else {
                    panic!("last returned nothing");
                }
            } else {
                if let Some(artist) = self.artists.get(i - 1) {
                    //if we can update the selected artist
                    self.selected_artist = artist.clone();
                } else {
                    panic!("could not get i - 1");
                }
            }
        } else {
            panic!("no selected artist?");
        }
    }
    pub fn albums_up(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_album() {
            //try to move up
            if i == 0 {
                if let Some(album) = self.selected_artist.albums.last() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album = album.clone();
                } else {
                    panic!("no artists?");
                }
            } else {
                if let Some(album) = self.selected_artist.albums.get(i - 1) {
                    //if we can update the selected artist
                    self.selected_artist.selected_album = album.clone();
                } else {
                    panic!("could not get i - 1");
                }
            }
        } else {
            panic!("no selected artist?");
        }
    }
    pub fn albums_down(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_album() {
            //try to move up
            if let Some(album) = self.selected_artist.albums.get(i + 1) {
                //if we can update the selected artist
                self.selected_artist.selected_album = album.clone();
            } else {
                if let Some(album) = self.selected_artist.albums.first() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album = album.clone();
                } else {
                    panic!("first returned nothing");
                }
            }
        } else {
            panic!("no selected album?");
        }
    }
    pub fn songs_up(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_song() {
            //try to move up
            if i == 0 {
                if let Some(song) = self.selected_artist.selected_album.songs.last() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album.selected_song = song.clone();
                } else {
                    //TODO: if there are no artists set to none
                    // self.selected_artist = None;
                    panic!("no artists?");
                }
            } else {
                if let Some(song) = self.selected_artist.selected_album.songs.get(i - 1) {
                    //if we can update the selected artist
                    self.selected_artist.selected_album.selected_song = song.clone();
                } else {
                    panic!("could not get i - 1");
                }
            }
        } else {
            panic!("no selected song?");
        }
    }
    pub fn songs_down(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_song() {
            //try to move up
            if let Some(song) = self.selected_artist.selected_album.songs.get(i + 1) {
                //if we can update the selected artist
                self.selected_artist.selected_album.selected_song = song.clone();
            } else {
                if let Some(song) = self.selected_artist.selected_album.songs.first() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album.selected_song = song.clone();
                } else {
                    panic!("first returned nothing");
                }
            }
        } else {
            panic!("no selected song?");
        }
    }

    pub fn artist_names(&self) -> Vec<String> {
        self.artists.iter().map(|a| a.name.clone()).collect()
    }
    pub fn album_names(&self) -> Vec<String> {
        self.selected_artist
            .albums
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }
    pub fn song_names(&self) -> Vec<String> {
        self.selected_artist
            .selected_album
            .songs
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }
    pub fn get_selected_artist(&self) -> Option<usize> {
        for (i, artist) in self.artists.iter().enumerate() {
            if artist == &self.selected_artist {
                return Some(i);
            }
        }
        None
    }
    pub fn get_selected_album(&self) -> Option<usize> {
        for (i, album) in self.selected_artist.albums.iter().enumerate() {
            if album == &self.selected_artist.selected_album {
                return Some(i);
            }
        }
        None
    }
    pub fn get_selected_song(&self) -> Option<usize> {
        for (i, album) in self.selected_artist.selected_album.songs.iter().enumerate() {
            if album == &self.selected_artist.selected_album.selected_song {
                return Some(i);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
    pub selected_album: Album,
}

impl PartialEq for Artist {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.albums == other.albums
        // && self.selected_album == other.selected_album
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub total_discs: u16,
    pub songs: Vec<Song>,
    pub selected_song: Song,
}
impl PartialEq for Album {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.artist == other.artist
        // && self.selected_song == other.selected_song
    }
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
        if let Ok(tag) = Tag::new().read_from_path(&path) {
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
        } else {
            Song {
                name: String::new(),
                name_with_number: String::new(),
                track_number: 1,
                album: String::new(),
                album_artist: String::new(),
                year: 0,
                path,
                disc: 0,
                total_disc: 0,
                duration: 0.0,
                elapsed: None,
            }
        }
    }

    pub fn update(&mut self, elapsed: f64, duration: f64) {
        self.elapsed = Some(elapsed);
        self.duration = duration;
    }
}
impl PartialEq for Song {
    fn eq(&self, other: &Song) -> bool {
        self.name == other.name && self.path == other.path
    }
}

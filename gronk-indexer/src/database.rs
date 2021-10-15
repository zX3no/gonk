use audiotags::Tag;
use hashbrown::HashMap;
use jwalk::WalkDir;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

type Ar = HashMap<String, Al>;
type Al = HashMap<String, So>;
type So = HashMap<String, Song>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Database {
    artists: Vec<Artist>,
}

impl Database {
    pub fn create_map(path: &str) -> Ar {
        let path = Path::new(path);
        let mut songs: So = HashMap::new();
        for entry in WalkDir::new(path).into_iter().flatten() {
            if let Some(ex) = entry.path().extension() {
                if ex == "flac" || ex == "mp3" || ex == "m4a" || ex == "wav" {
                    let song = Song::from(entry.path());

                    songs.insert(song.name.clone(), song);
                }
            }
        }

        let mut albums: Al = HashMap::new();
        for (k, v) in &songs {
            if albums.get(&v.album).is_some() {
                // albums.get_mut(&v.album).unwrap().songs.push(v.clone());
            } else {
                let mut al = HashMap::new();
                al.insert(k.clone(), v.clone()).unwrap();
                albums.insert(v.album.to_string(), al);
            }
        }

        dbg!(albums);

        return Ar::default();
    }
    pub fn create(path: &str) -> Self {
        let path = Path::new(path);

        let mut songs: HashMap<String, Song> = HashMap::new();
        for entry in WalkDir::new(path).into_iter().flatten() {
            if let Some(ex) = entry.path().extension() {
                if ex == "flac" || ex == "mp3" || ex == "m4a" || ex == "wav" {
                    let song = Song::from(entry.path());

                    songs.insert(song.name.clone(), song);
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

        let artists = artists.values().cloned().collect();

        Self { artists }
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

        let year = tag.year().unwrap_or(0);

        Song {
            name,
            name_with_number,
            track_number,
            album: tag.album_title().unwrap().to_string(),
            album_artist,
            //todo
            duration: 0,
            year,
            path,
            disc,
            total_disc,
        }
    }
}

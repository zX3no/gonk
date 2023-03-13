//! Music Playlists
//!
//! Each playlist has it's own file.
//!
use crate::{database_path, escape, Index, Song};
use core::fmt;
use std::{
    fs::{self},
    path::PathBuf,
    str::FromStr,
};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct Playlist {
    pub songs: Index<Song>,
    //Keep private so everything is escaped properly.
    name: String,
    path: PathBuf,
}

impl Playlist {
    pub fn new(name: &str, songs: Vec<Song>) -> Self {
        let name = escape(name);

        let mut path = database_path();
        path.pop();
        path.push(format!("{name}.playlist"));

        Self {
            path,
            name,
            songs: Index::from(songs),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn save(&self) -> std::io::Result<()> {
        fs::write(&self.path, self.to_string())
    }
    pub fn delete(&self) -> std::io::Result<()> {
        fs::remove_file(&self.path)
    }
}

impl fmt::Display for Playlist {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let songs: String = self.songs.iter().map(|song| song.to_string()).collect();
        write!(
            f,
            "{}\t{}\t\n{}",
            self.name,
            self.path.to_str().unwrap(),
            songs
        )
    }
}

impl FromStr for Playlist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('\n').collect();
        let (name, path) = parts[0].split_once('\t').unwrap();

        let songs: Vec<Song> = parts[1..]
            .iter()
            .map(|string| string.parse::<Song>().unwrap())
            .collect();

        Ok(Self {
            name: name.to_string(),
            path: PathBuf::from(path),
            songs: Index::from(songs),
        })
    }
}

pub fn playlists() -> Vec<Playlist> {
    let mut path = database_path();
    path.pop();

    WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|path| match path.path().extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("playlist"))
            }
            None => false,
        })
        .flat_map(|entry| fs::read_to_string(entry.path()))
        .map(|string| string.parse::<Playlist>().unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist() {
        let playlist = Playlist::new("name", vec![Song::example(), Song::example()]);
        let string = playlist.to_string();
        let _ = string.parse::<Playlist>().unwrap();
    }

    #[test]
    fn save() {
        let playlist = Playlist::new(
            "test",
            vec![
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
                Song::example(),
            ],
        );
        playlist.save().unwrap();
        let playlists = playlists();
        assert!(!playlists.is_empty());
        playlist.delete().unwrap();
    }
}

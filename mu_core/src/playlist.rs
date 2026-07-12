//! Music Playlists
//!
//! Each playlist has it's own file.
//!
use crate::{Deserialize, Index, Serialize, Song, escape};
use std::{
    fs::{self},
    path::{Path, PathBuf},
};

#[derive(Debug, Default, PartialEq)]
pub struct Playlist {
    name: String,
    path: PathBuf,
    pub songs: Index<Song>,
}

impl Playlist {
    pub fn new(name: &str, songs: Vec<Song>, config_path: &Path) -> Self {
        let name = escape(name);
        Self {
            path: config_path.join(format!("{name}.playlist")),
            name: String::from(name),
            songs: Index::from(songs),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn save(&self) -> std::io::Result<()> {
        fs::write(&self.path, self.serialize())
    }
    pub fn delete(&self) {
        minbin::trash(&self.path).unwrap();
    }
}

impl Serialize for Playlist {
    fn serialize(&self) -> String {
        let mut buffer = String::new();
        buffer.push_str(&self.name);
        buffer.push('\t');
        buffer.push_str(self.path.to_str().unwrap());
        buffer.push('\n');
        buffer.push_str(&self.songs.serialize());
        buffer
    }
}

impl Deserialize for Playlist {
    type Error = Box<dyn std::error::Error>;

    fn deserialize(s: &str) -> Result<Self, Self::Error> {
        let (start, end) = s.split_once('\n').ok_or("Invalid playlist")?;
        let (name, path) = start.split_once('\t').ok_or("Invalid playlsit")?;

        Ok(Self {
            name: name.to_string(),
            path: PathBuf::from(path),
            songs: Index::from(Vec::<Song>::deserialize(end)?),
        })
    }
}

pub fn playlists(config_path: &Path) -> Vec<Playlist> {
    winwalk::walkdir(config_path.to_str().unwrap(), 0)
        .into_iter()
        .flatten()
        .filter(|entry| match entry.extension() {
            Some(ex) => {
                matches!(ex.to_str(), Some("playlist"))
            }
            None => false,
        })
        .flat_map(|entry| fs::read_to_string(entry.path))
        .map(|string| Playlist::deserialize(&string).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn playlist() {
        let config = config_paths();
        let playlist = Playlist::new("name", vec![Song::example(), Song::example()], &config.mu);
        let string = playlist.serialize();
        let p = Playlist::deserialize(&string).unwrap();
        assert_eq!(playlist, p);
    }

    #[test]
    fn save() {
        let config = config_paths();
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
            &config.mu,
        );
        playlist.save().unwrap();
        let playlists = playlists(&config.mu);
        assert!(!playlists.is_empty());
        playlist.delete();
    }
}

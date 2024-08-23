//! Music Playlists
//!
//! Each playlist has it's own file.
//!
use crate::{escape, gonk_path, Deserialize, Index, Serialize, Song};
use std::{
    fs::{self},
    path::PathBuf,
};

#[derive(Debug, Default, PartialEq)]
pub struct Playlist {
    name: String,
    path: PathBuf,

    pub songs: Index<Song>,
}

impl Playlist {
    pub fn new(name: &str, songs: Vec<Song>) -> Self {
        let name = escape(name);
        Self {
            path: gonk_path().join(format!("{name}.playlist")),
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
    //TODO: This is super slow.
    pub fn delete(&self) {
        mini::trash(&self.path).unwrap();
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

pub fn playlists() -> Vec<Playlist> {
    winwalk::walkdir(gonk_path().to_str().unwrap(), 0)
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

    #[test]
    fn playlist() {
        let playlist = Playlist::new("name", vec![Song::example(), Song::example()]);
        let string = playlist.serialize();
        let p = Playlist::deserialize(&string).unwrap();
        assert_eq!(playlist, p);
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
        playlist.delete();
    }
}

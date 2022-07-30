use walkdir::WalkDir;

use crate::{database_path, Index, RawSong, SONG_LEN};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    str::from_utf8_unchecked,
};
//Do i want to store the file handles?
//How will remove items from the playlist? Override the file. replace the song with zeroes leaving gaps that will need to be cleaned up???
//I guess file writes can run along side the program since we've already got all the data loaded.

//Open the file in append mode for adding songs on the end.
//Will I need to use different file handles when appending, deleting or overriding
pub fn playlists() -> Vec<RawPlaylist> {
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
        .flat_map(|entry| fs::read(entry.path()))
        .map(|bytes| RawPlaylist::from(bytes.as_slice()))
        .collect()
}

pub fn remove_playlist(path: &Path) {
    fs::remove_file(path).unwrap();
}

#[derive(Debug)]
pub struct RawPlaylist {
    pub name: String,
    pub path: PathBuf,
    pub songs: Index<RawSong>,
}

impl RawPlaylist {
    pub fn new(name: &str, data: Vec<RawSong>) -> Self {
        let mut path = database_path();
        path.pop();
        path.push(format!("{}.playlist", name));

        Self {
            path,
            name: name.to_string(),
            songs: Index::from(data),
        }
    }
    pub fn save(&self) {
        //Delete the contents of the file and overwrite with new settings.
        let file = File::create(&self.path).unwrap();
        let mut writer = BufWriter::new(file);

        //Convert to bytes.
        let mut bytes = Vec::new();
        bytes.extend((self.name.len() as u16).to_le_bytes());
        bytes.extend(self.name.as_bytes());
        for song in &self.songs.data {
            bytes.extend(song.into_bytes());
        }

        writer.write_all(&bytes).unwrap();
        writer.flush().unwrap();
    }
}

impl From<&[u8]> for RawPlaylist {
    fn from(bytes: &[u8]) -> Self {
        let name_len = u16::from_le_bytes(bytes[0..2].try_into().unwrap()) as usize;
        let name = unsafe { from_utf8_unchecked(&bytes[2..name_len + 2]) };

        //TODO: is it +3 or +2?
        let mut i = name_len + 2;
        let mut songs = Vec::new();

        while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
            songs.push(RawSong::from(bytes));
            i += SONG_LEN;
        }

        let mut path = database_path();
        path.pop();
        path.push(format!("{}.playlist", name));

        Self {
            name: name.to_string(),
            path,
            songs: Index::from(songs),
        }
    }
}

use crate::{database_path, raw_song::*, Index, Song, SONG_LEN};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    str::from_utf8_unchecked,
};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct RawPlaylist {
    pub name: String,
    pub path: PathBuf,
    pub songs: Index<RawSong>,
}

impl RawPlaylist {
    pub fn new(name: &str, songs: Vec<Song>) -> Self {
        let mut path = database_path();
        path.pop();
        path.push(format!("{name}.playlist"));

        let songs: Vec<RawSong> = songs.into_iter().map(|song| RawSong::from(&song)).collect();

        Self {
            path,
            name: name.to_string(),
            songs: Index::from(songs),
        }
    }
    pub fn extend<I: IntoIterator<Item = Song>>(&mut self, iter: I) {
        let iter = iter.into_iter().map(|song| RawSong::from(&song));
        self.songs.data.extend(iter);
    }
    pub fn extend_raw<I: IntoIterator<Item = RawSong>>(&mut self, iter: I) {
        self.songs.data.extend(iter);
    }
    //TODO: Why is the file handle being reopened so much.
    pub fn save(&self) {
        //Delete the contents of the file and overwrite with new settings.
        let file = File::create(&self.path).unwrap();
        let mut writer = BufWriter::new(file);

        //Convert to bytes.
        let mut bytes = Vec::new();
        bytes.extend((self.name.len() as u16).to_le_bytes());
        bytes.extend(self.name.as_bytes());
        for song in &self.songs.data {
            bytes.extend(song.as_bytes());
        }

        writer.write_all(&bytes).unwrap();
        writer.flush().unwrap();
    }
    pub fn delete(&self) {
        fs::remove_file(&self.path).unwrap();
    }
}

impl From<&[u8]> for RawPlaylist {
    fn from(bytes: &[u8]) -> Self {
        unsafe {
            let name_len = u16::from_le_bytes(bytes[0..2].try_into().unwrap()) as usize;
            let name = from_utf8_unchecked(&bytes[2..name_len + 2]);

            let mut i = name_len + 2;
            let mut songs = Vec::new();

            while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
                songs.push(RawSong::from(bytes));
                i += SONG_LEN;
            }

            let mut path = database_path();
            path.pop();
            path.push(format!("{name}.playlist"));

            Self {
                name: name.to_string(),
                path,
                songs: Index::from(songs),
            }
        }
    }
}

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

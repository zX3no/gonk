//! Muisc Playlists
//!
//! Each playlist has it's own file.
//!
use crate::{database_path, Index, Song};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    str::from_utf8_unchecked,
};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct Playlist {
    pub name: String,
    pub path: PathBuf,
    pub songs: Index<Song>,
}

pub fn new(name: &str, songs: Vec<Song>) -> Playlist {
    let mut path = database_path();
    path.pop();
    path.push(format!("{name}.playlist"));

    Playlist {
        path,
        name: name.to_string(),
        songs: Index::from(songs),
    }
}

fn from_slice(bytes: &[u8]) -> Result<Playlist, Box<dyn Error + Send + Sync>> {
    // unsafe {
    //     let name_len = u16::from_le_bytes(bytes[0..2].try_into().unwrap()) as usize;
    //     let name = from_utf8_unchecked(&bytes[2..name_len + 2]);

    //     let mut i = name_len + 2;
    //     let mut songs = Vec::new();

    //     while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
    //         songs.push(bytes_to_song(bytes)?);
    //         i += SONG_LEN;
    //     }

    //     let mut path = database_path();
    //     path.pop();
    //     path.push(format!("{name}.playlist"));

    //     Ok(Playlist {
    //         name: name.to_string(),
    //         path,
    //         songs: Index::from(songs),
    //     })
    // }
    todo!()
}

// pub fn extend<I: IntoIterator<Item = Song>>(&mut self, iter: I) {
//     let iter = iter.into_iter().map(|song| RawSong::from(&song));
//     self.songs.data.extend(iter);
// }
// pub fn extend_raw<I: IntoIterator<Item = RawSong>>(&mut self, iter: I) {
//     self.songs.data.extend(iter);
// }

//TODO: Why is the file handle being reopened so much.
pub fn save(playlist: &Playlist) -> std::io::Result<()> {
    //Delete the contents of the file and overwrite with new settings.
    let file = File::create(&playlist.path)?;
    let mut writer = BufWriter::new(file);

    //Convert to bytes.
    let mut bytes = Vec::new();
    bytes.extend((playlist.name.len() as u16).to_le_bytes());
    bytes.extend(playlist.name.as_bytes());
    for song in playlist.songs.iter() {
        // bytes.extend(song.to_bytes());
        todo!();
    }

    writer.write_all(&bytes)?;
    writer.flush()?;

    Ok(())
}

pub fn delete(playlist: &Playlist) -> std::io::Result<()> {
    fs::remove_file(&playlist.path)
}

pub fn playlists() -> Result<Vec<Playlist>, Box<dyn Error + Send + Sync>> {
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
        .map(|bytes| from_slice(bytes.as_slice()))
        .collect()
}

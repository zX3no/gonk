use std::{collections::HashMap, path::Path};

use jwalk::WalkDir;

use crate::database::{Album, Artist, Song};

pub fn get_artists() -> HashMap<String, Artist> {
    let path = Path::new(r"D:\OneDrive\Music");

    let mut songs = Vec::new();

    for entry in WalkDir::new(path) {
        if let Ok(e) = entry {
            if let Some(ex) = e.path().extension() {
                if ex == "flac" {
                    let song = Song::from(e.path());

                    songs.push(song);
                }
            }
        }
    }

    //create a hashmap of albums
    //todo change to hashbrown
    let mut albums: HashMap<String, Album> = HashMap::new();

    for song in songs {
        if albums.get(&song.album).is_some() {
            albums.get_mut(&song.album).unwrap().songs.push(song);
        } else {
            albums.insert(
                song.album.to_string(),
                Album {
                    title: song.album.clone(),
                    artist: song.album_artist.clone(),
                    songs: vec![song],
                },
            );
        }
    }

    let mut artists: HashMap<String, Artist> = HashMap::new();

    for album in albums {
        let (_, v) = album;

        if artists.get(&v.artist).is_some() {
            artists.get_mut(&v.artist).unwrap().albums.push(v);
        } else {
            artists.insert(
                v.artist.clone(),
                Artist {
                    name: v.artist.clone(),
                    albums: vec![v],
                },
            );
        }
    }

    return artists;
}

use std::{collections::HashMap, path::Path};

use jwalk::WalkDir;

mod database;
use database::album::Album;
use database::song::Song;

fn main() {
    let path = Path::new(r"D:\OneDrive\Music\");

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
    let mut map: HashMap<String, Album> = HashMap::new();

    for song in songs {
        let a = song.album.as_str();

        if map.get(a).is_some() {
            map.get_mut(a).unwrap().songs.push(song);
        } else {
            let mut v = Vec::new();
            v.push(song.clone());
            let album = map.insert(
                a.to_string(),
                Album {
                    title: String::from(a),
                    songs: v,
                },
            );
        }
    }

    dbg!(&map);
}

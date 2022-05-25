pub use crate::{
    index::Index,
    song::Song,
    sqlite::{Database, State},
    toml::{Bind, Colors, GlobalHotkey, Hotkey, Key, Modifier, Toml},
};
use static_init::dynamic;
use std::path::PathBuf;

mod index;
mod song;
mod toml;

pub mod sqlite;

#[dynamic]
pub static GONK_DIR: PathBuf = {
    let gonk = dirs::config_dir().unwrap().join("gonk");
    if !gonk.exists() {
        std::fs::create_dir_all(&gonk).unwrap();
    }
    gonk
};

#[dynamic]
pub static DB_DIR: PathBuf = GONK_DIR.join("gonk.db");

#[dynamic]
pub static TOML_DIR: PathBuf = GONK_DIR.join("gonk.toml");

#[cfg(test)]
mod tests {
    /*
        cargo test --release --lib -- tests::bench_adding --exact
        hyperfine 'cargo test --release --lib -- tests::bench_adding --exact' -w 5 -r 50
    */

    #[test]
    fn bench_adding() {
        use crate::sqlite;
        use crate::sqlite::conn;
        use crate::Song;
        use jwalk::WalkDir;
        use rayon::iter::{IntoParallelIterator, ParallelIterator};
        use std::path::PathBuf;

        unsafe {
            sqlite::CONN = sqlite::open_database();
        }

        let paths: Vec<PathBuf> = WalkDir::new("D:\\Music")
            .into_iter()
            .flatten()
            .map(|dir| dir.path())
            .filter(|path| match path.extension() {
                Some(ex) => {
                    matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
                }
                None => false,
            })
            .collect();

        let songs: Vec<Song> = paths
            .into_par_iter()
            .map(|dir| Song::from(&dir))
            .flatten()
            .collect();

        if songs.is_empty() {
            return;
        }

        let mut stmt = String::from("BEGIN;\n");
        stmt.push_str(&songs.iter()
                .map(|song| {
                    let artist = song.artist.replace('\'', r"''");
                    let album = song.album.replace('\'', r"''");
                    let name = song.name.replace('\'', r"''");
                    let path = song.path.to_string_lossy().replace('\'', r"''");
                    let parent = "D:\\Music";
                    format!("INSERT OR IGNORE INTO song (number, disc, name, album, artist, path, duration, track_gain, parent) VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}');",
                                song.number, song.disc, name, album, artist,path, song.duration.as_secs_f64(), song.track_gain, parent)
                })
                .collect::<Vec<_>>().join("\n"));

        stmt.push_str("COMMIT;\n");

        conn().execute_batch(&stmt).unwrap();
    }

    /*
    cargo test --release --lib -- tests::bench_query --exact --nocapture
    */

    #[test]
    fn bench_query() {
        use crate::sqlite;

        unsafe {
            sqlite::CONN = sqlite::open_database();
        }

        let t = std::time::Instant::now();

        for _ in 0..10 {
            let artists = sqlite::get_all_artists();
            for artist in artists {
                let albums = sqlite::get_all_albums_by_artist(&artist);
                for album in albums {
                    let songs = sqlite::get_all_songs_from_album(&album, &artist);
                    assert!(!songs.is_empty());
                }
            }
        }
        eprintln!("{:0.2?}", t.elapsed())
    }

    #[cfg(windows)]
    #[test]
    fn windows_keybindings() {
        use crate::{Bind, Key, Modifier};
        use global_hotkeys::{keys, modifiers};

        let b = Bind {
            key: Key::from("A"),
            modifiers: Some(vec![Modifier::ALT, Modifier::SHIFT]),
        };
        assert_eq!(Bind::new("A").key(), 'A' as u32);
        assert_eq!(Bind::new("TAB").key(), keys::TAB);
        assert_eq!(b.modifiers(), modifiers::ALT | modifiers::SHIFT);
    }
}

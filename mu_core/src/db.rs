use crate::*;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::File,
    io::{BufWriter, Write},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub disc_number: u8,
    pub track_number: u8,
    pub path: String,
    pub gain: f32,
    pub year: u16,
}

impl Serialize for Song {
    fn serialize(&self) -> String {
        use std::fmt::Write;

        let mut buffer = String::new();
        let gain = if self.gain == 0.0 {
            "0.0".to_string()
        } else {
            self.gain.to_string()
        };

        let result = writeln!(
            &mut buffer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            escape(&self.title),
            escape(&self.album),
            escape(&self.artist),
            self.disc_number,
            self.track_number,
            escape(&self.path),
            gain,
            self.year,
        );

        match result {
            Ok(_) => buffer,
            Err(err) => panic!("{err} failed to write song: {:?}", self),
        }
    }
}

impl Deserialize for Song {
    type Error = Box<dyn std::error::Error>;

    fn deserialize(s: &str) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err("Empty song")?;
        }

        //`file.lines()` will not include newlines
        //but song.to_string() will.
        let s = if s.as_bytes().last() == Some(&b'\n') {
            &s[..s.len() - 1]
        } else {
            s
        };

        let mut parts = s.split('\t');
        Ok(Song {
            title: parts.next().ok_or("Missing title")?.to_string(),
            album: parts.next().ok_or("Missing album")?.to_string(),
            artist: parts.next().ok_or("Missing artist")?.to_string(),
            disc_number: parts.next().ok_or("Missing disc_number")?.parse::<u8>()?,
            track_number: parts.next().ok_or("Missing track_number")?.parse::<u8>()?,
            path: parts.next().ok_or("Missing path")?.to_string(),
            gain: parts.next().ok_or("Missing gain")?.parse::<f32>()?,
            year: parts
                .next()
                .and_then(|y| y.parse::<u16>().ok())
                .unwrap_or(0),
        })
    }
}

impl Serialize for Vec<Song> {
    fn serialize(&self) -> String {
        let mut buffer = String::new();
        for song in self {
            buffer.push_str(&song.serialize());
        }
        buffer
    }
}

impl Deserialize for Vec<Song> {
    type Error = Box<dyn std::error::Error>;

    fn deserialize(s: &str) -> Result<Self, Self::Error> {
        s.trim().split('\n').map(Song::deserialize).collect()
    }
}

impl Song {
    pub fn new() -> Self {
        Self {
            title: "Unknown Title".to_string(),
            album: "Unknown Album".to_string(),
            artist: "Unknown Artist".to_string(),
            disc_number: 1,
            track_number: 1,
            path: String::new(),
            gain: 0.0,
            year: 0,
        }
    }
    pub fn example() -> Self {
        Self {
            title: "title".to_string(),
            album: "album".to_string(),
            artist: "artist".to_string(),
            disc_number: 1,
            track_number: 1,
            path: "path".to_string(),
            gain: 1.0,
            year: 0,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

impl Album {
    pub fn year(&self) -> u16 {
        self.songs
            .iter()
            .map(|s| s.year)
            .find(|&y| y != 0)
            .unwrap_or(0)
    }
}

#[derive(Debug, Default)]
pub struct Artist {
    pub albums: Vec<Album>,
}

impl TryFrom<&Path> for Song {
    type Error = String;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        //TODO: Two different song implementations?
        //I feel like the decoder stuff belongs in the playback library.
        //But something just feels weird about this.
        let osong = onmi::metadata(path, false)?;
        Ok(Song {
            title: osong.title,
            album: osong.album,
            artist: osong.artist,
            disc_number: osong.disc_number,
            track_number: osong.track_number,
            path: osong.path,
            gain: osong.gain,
            year: osong.year,
        })
    }
}

#[derive(Debug)]
pub enum ScanResult {
    Completed {
        elapsed: Duration,
        tracks: usize,
    },
    CompletedWithErrors {
        elapsed: Duration,
        tracks: usize,
        errors: Vec<String>,
    },
    FileInUse,
}

pub fn reset(config: &Config) -> Result<(), Box<dyn Error>> {
    fs::remove_file(&config.settings)?;
    if config.database.exists() {
        fs::remove_file(&config.database)?;
    }
    Ok(())
}

pub fn create(music_dir: &str, config_database_path: PathBuf) -> JoinHandle<ScanResult> {
    let path = music_dir.to_string();
    thread::spawn(move || {
        let started = Instant::now();
        let mut db_path = config_database_path.to_path_buf();
        db_path.pop();
        db_path.push("temp.db");

        match File::create(&db_path) {
            Ok(file) => {
                let paths: Vec<winwalk::DirEntry> = winwalk::walkdir(path, 0)
                    .into_iter()
                    .flatten()
                    .filter(|entry| match entry.extension() {
                        Some(ex) => {
                            matches!(
                                ex.to_str().map(|s| s.to_ascii_lowercase()).as_deref(),
                                Some("flac" | "mp3" | "ogg")
                            )
                        }
                        None => false,
                    })
                    .collect();

                let songs: Vec<_> = paths
                    .into_par_iter()
                    .map(|entry| Song::try_from(Path::new(&entry.path)))
                    .collect();

                let errors: Vec<String> = songs
                    .iter()
                    .filter_map(|song| {
                        if let Err(err) = song {
                            Some(err.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                let songs: Vec<Song> = songs.into_iter().flatten().collect();
                let tracks = songs.len();
                let mut writer = BufWriter::new(&file);
                writer.write_all(&songs.serialize().into_bytes()).unwrap();
                writer.flush().unwrap();

                //Remove old database and replace it with new.
                fs::rename(db_path, config_database_path).unwrap();

                let elapsed = started.elapsed();
                if errors.is_empty() {
                    ScanResult::Completed { elapsed, tracks }
                } else {
                    ScanResult::CompletedWithErrors {
                        elapsed,
                        tracks,
                        errors,
                    }
                }
            }
            Err(_) => ScanResult::FileInUse,
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{str::from_utf8_unchecked, time::Duration};

    use super::*;

    #[test]
    fn string() {
        let song = Song::example();
        let string = song.serialize();
        assert_eq!(Song::deserialize(&string).unwrap(), song);
    }

    #[test]
    fn path() {
        let path = PathBuf::from(
            r"D:\OneDrive\Music\Mouse On The Keys\an anxious object\04. dirty realism.flac",
        );
        let _ = Song::try_from(path.as_path()).unwrap();
    }

    #[test]
    fn database() {
        let config = config_paths();
        let handle = create("D:\\OneDrive\\Music", config.database.clone());

        while !handle.is_finished() {
            thread::sleep(Duration::from_millis(1));
        }
        handle.join().unwrap();
        let bytes = fs::read(&config.database).unwrap();
        let db: Result<Vec<Song>, Box<dyn Error>> = unsafe { from_utf8_unchecked(&bytes) }
            .lines()
            .map(Song::deserialize)
            .collect();
        let _ = db.unwrap();
    }
}

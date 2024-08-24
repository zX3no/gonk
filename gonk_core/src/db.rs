use crate::*;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::File,
    io::{BufWriter, Write},
    thread::{self, JoinHandle},
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
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            escape(&self.title),
            escape(&self.album),
            escape(&self.artist),
            self.disc_number,
            self.track_number,
            escape(&self.path),
            gain,
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

pub const UNKNOWN_TITLE: &str = "Unknown Title";
pub const UNKNOWN_ALBUM: &str = "Unknown Album";
pub const UNKNOWN_ARTIST: &str = "Unknown Artist";

impl Song {
    pub fn default() -> Self {
        Self {
            title: UNKNOWN_TITLE.to_string(),
            album: UNKNOWN_ALBUM.to_string(),
            artist: UNKNOWN_ARTIST.to_string(),
            disc_number: 1,
            track_number: 1,
            path: String::new(),
            gain: 0.0,
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
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

#[derive(Debug, Default)]
pub struct Artist {
    pub albums: Vec<Album>,
}

impl TryFrom<&Path> for Song {
    type Error = String;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let extension = path.extension().ok_or("Path is not audio")?;

        if extension != "flac" {
            use symphonia::{
                core::{formats::FormatOptions, io::*, meta::*, probe::Hint},
                default::get_probe,
            };

            let file = match File::open(path) {
                Ok(file) => file,
                Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
            };

            let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

            let mut probe = match get_probe().format(
                &Hint::new(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions {
                    limit_visual_bytes: Limit::Maximum(1),
                    ..Default::default()
                },
            ) {
                Ok(probe) => probe,
                Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy()))?,
            };

            let mut title = String::from("Unknown Title");
            let mut album = String::from("Unknown Album");
            let mut artist = String::from("Unknown Artist");
            let mut track_number = 1;
            let mut disc_number = 1;
            let mut gain = 0.0;

            let mut metadata_revision = probe.format.metadata();
            let mut metadata = probe.metadata.get();
            let mut m = None;

            if let Some(metadata) = metadata_revision.skip_to_latest() {
                m = Some(metadata);
            };

            if let Some(metadata) = &mut metadata {
                if let Some(metadata) = metadata.skip_to_latest() {
                    m = Some(metadata)
                };
            }

            if let Some(metadata) = m {
                for tag in metadata.tags() {
                    if let Some(std_key) = tag.std_key {
                        match std_key {
                            StandardTagKey::AlbumArtist => artist = tag.value.to_string(),
                            StandardTagKey::Artist if artist == "Unknown Artist" => {
                                artist = tag.value.to_string()
                            }
                            StandardTagKey::Album => album = tag.value.to_string(),
                            StandardTagKey::TrackTitle => title = tag.value.to_string(),
                            StandardTagKey::TrackNumber => {
                                let num = tag.value.to_string();
                                if let Some((num, _)) = num.split_once('/') {
                                    track_number = num.parse().unwrap_or(1);
                                } else {
                                    track_number = num.parse().unwrap_or(1);
                                }
                            }
                            StandardTagKey::DiscNumber => {
                                let num = tag.value.to_string();
                                if let Some((num, _)) = num.split_once('/') {
                                    disc_number = num.parse().unwrap_or(1);
                                } else {
                                    disc_number = num.parse().unwrap_or(1);
                                }
                            }
                            StandardTagKey::ReplayGainTrackGain => {
                                let tag = tag.value.to_string();
                                let (_, value) =
                                    tag.split_once(' ').ok_or("Invalid replay gain.")?;
                                let db = value.parse().unwrap_or(0.0);
                                gain = 10.0f32.powf(db / 20.0);
                            }
                            _ => (),
                        }
                    }
                }
            }

            Ok(Song {
                title,
                album,
                artist,
                disc_number,
                track_number,
                path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                gain,
            })
        } else {
            read_metadata(path)
                .map_err(|err| format!("Error: ({err}) @ {}", path.to_string_lossy()))
        }
    }
}

#[derive(Debug)]
pub enum ScanResult {
    Completed,
    CompletedWithErrors(Vec<String>),
    FileInUse,
}

pub fn reset() -> Result<(), Box<dyn Error>> {
    fs::remove_file(settings_path())?;
    if database_path().exists() {
        fs::remove_file(database_path())?;
    }
    Ok(())
}

pub fn create(path: &str) -> JoinHandle<ScanResult> {
    let path = path.to_string();
    thread::spawn(move || {
        let mut db_path = database_path().to_path_buf();
        db_path.pop();
        db_path.push("temp.db");

        match File::create(&db_path) {
            Ok(file) => {
                let paths: Vec<winwalk::DirEntry> = winwalk::walkdir(path, 0)
                    .into_iter()
                    .flatten()
                    .filter(|entry| match entry.extension() {
                        Some(ex) => {
                            matches!(ex.to_str(), Some("flac" | "mp3" | "ogg"))
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
                let mut writer = BufWriter::new(&file);
                writer.write_all(&songs.serialize().into_bytes()).unwrap();
                writer.flush().unwrap();

                //Remove old database and replace it with new.
                fs::rename(db_path, database_path()).unwrap();

                // let _db = vdb::create().unwrap();

                if errors.is_empty() {
                    ScanResult::Completed
                } else {
                    ScanResult::CompletedWithErrors(errors)
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
        let handle = create("D:\\OneDrive\\Music");

        while !handle.is_finished() {
            thread::sleep(Duration::from_millis(1));
        }
        handle.join().unwrap();
        let bytes = fs::read(database_path()).unwrap();
        let db: Result<Vec<Song>, Box<dyn Error>> = unsafe { from_utf8_unchecked(&bytes) }
            .lines()
            .map(Song::deserialize)
            .collect();
        let _ = db.unwrap();
    }
}

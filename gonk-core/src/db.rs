use crate::database_path;
use crate::*;
use core::fmt;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    str::from_utf8_unchecked,
    thread::{self, JoinHandle},
};
use walkdir::{DirEntry, WalkDir};

pub static mut LEN: usize = 0;

#[derive(Debug, Clone, PartialEq, Default)]
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
        let mut buffer = String::new();
        buffer.push_str(&escape(&self.title));
        buffer.push('\t');
        buffer.push_str(&escape(&self.album));
        buffer.push('\t');
        buffer.push_str(&escape(&self.artist));
        buffer.push('\t');
        buffer.push_str(&self.disc_number.to_string());
        buffer.push('\t');
        buffer.push_str(&self.track_number.to_string());
        buffer.push('\t');
        buffer.push_str(&escape(&self.path));
        buffer.push('\t');
        let gain = if self.gain == 0.0 {
            String::from("0.0")
        } else {
            self.gain.to_string()
        };
        buffer.push_str(&gain);
        buffer.push('\n');
        buffer
    }
}

impl Deserialize for Song {
    type Error = Box<dyn std::error::Error>;

    fn deserialize(s: &str) -> Result<Self, Self::Error> {
        //`file.lines()` will not include newlines
        //but song.to_string() will.
        let s = if s.as_bytes().last() == Some(&b'\n') {
            &s[..s.len() - 1]
        } else {
            s
        };

        let mut song = Song::default();
        //I think this is a little faster than collecting.
        for (i, split) in s.split('\t').enumerate() {
            match i {
                0 => song.title = split.to_string(),
                1 => song.album = split.to_string(),
                2 => song.artist = split.to_string(),
                3 => song.disc_number = split.parse::<u8>()?,
                4 => song.track_number = split.parse::<u8>()?,
                5 => song.path = split.to_string(),
                6 => song.gain = split.parse::<f32>()?,
                _ => panic!("Invalid song format: {}", s),
            }
        }
        Ok(song)
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

#[derive(Debug, Default)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

#[derive(Debug, Default)]
pub struct Artist {
    pub albums: Vec<Album>,
}

impl fmt::Display for Song {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.title,
            self.album,
            self.artist,
            self.disc_number,
            self.track_number,
            self.path,
            self.gain
        )
    }
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

            //TODO: WTF IS THIS???
            let metadata = match metadata_revision.skip_to_latest() {
                Some(metadata) => metadata,
                None => match &mut metadata {
                    Some(metadata) => match metadata.skip_to_latest() {
                        Some(metadata) => metadata,
                        None => {
                            return Ok(Song {
                                title,
                                album,
                                artist,
                                disc_number,
                                track_number,
                                path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                                gain,
                            });
                        }
                    },
                    None => {
                        return Ok(Song {
                            title,
                            album,
                            artist,
                            disc_number,
                            track_number,
                            path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                            gain,
                        });
                    }
                },
            };

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
                            let (_, value) = tag.split_once(' ').ok_or("Invalid replay gain.")?;
                            let db = value.parse().unwrap_or(0.0);
                            gain = 10.0f32.powf(db / 20.0);
                        }
                        _ => (),
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
            match read_metadata(path) {
                Ok(metadata) => {
                    let track_number = metadata
                        .get("TRACKNUMBER")
                        .unwrap_or(&String::from("1"))
                        .parse()
                        .unwrap_or(1);

                    let disc_number = metadata
                        .get("DISCNUMBER")
                        .unwrap_or(&String::from("1"))
                        .parse()
                        .unwrap_or(1);

                    let mut gain = 0.0;
                    if let Some(db) = metadata.get("REPLAYGAIN_TRACK_GAIN") {
                        let g = db.replace(" dB", "");
                        if let Ok(db) = g.parse::<f32>() {
                            gain = 10.0f32.powf(db / 20.0);
                        }
                    }

                    let artist = match metadata.get("ALBUMARTIST") {
                        Some(artist) => artist.as_str(),
                        None => match metadata.get("ARTIST") {
                            Some(artist) => artist.as_str(),
                            None => "Unknown Artist",
                        },
                    };

                    let album = match metadata.get("ALBUM") {
                        Some(album) => album.as_str(),
                        None => "Unknown Album",
                    };

                    let title = match metadata.get("TITLE") {
                        Some(title) => title.as_str(),
                        None => "Unknown Title",
                    };

                    Ok(Song {
                        title: title.to_string(),
                        album: album.to_string(),
                        artist: artist.to_string(),
                        disc_number,
                        track_number,
                        path: path.to_str().ok_or("Invalid UTF-8 in path.")?.to_string(),
                        gain,
                    })
                }
                Err(err) => Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
            }
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

pub fn create(path: impl ToString) -> JoinHandle<ScanResult> {
    let path = path.to_string();
    thread::spawn(move || {
        let mut db_path = database_path();
        db_path.pop();
        db_path.push("temp.db");

        match File::create(&db_path) {
            Ok(file) => {
                let paths: Vec<DirEntry> = WalkDir::new(path)
                    .into_iter()
                    .flatten()
                    .filter(|path| match path.path().extension() {
                        Some(ex) => {
                            matches!(ex.to_str(), Some("flac" | "mp3" | "ogg"))
                        }
                        None => false,
                    })
                    .collect();

                let songs: Vec<_> = paths
                    .into_par_iter()
                    .map(|dir| Song::try_from(dir.path()))
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

pub fn read() -> Result<Vec<Song>, Box<dyn Error + Send + Sync>> {
    let bytes = match fs::read(database_path()) {
        Ok(bytes) => bytes,
        Err(error) => {
            return match error.kind() {
                std::io::ErrorKind::NotFound => Ok(Vec::new()),
                _ => Err(error)?,
            }
        }
    };

    let string = unsafe { from_utf8_unchecked(&bytes) };
    let songs: Vec<Song> = string
        .lines()
        .map(|line| Song::deserialize(line).unwrap())
        .collect();

    unsafe { LEN = songs.len() };

    Ok(songs)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn string() {
        let song = Song::example();
        let string = song.to_string();
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
        let _ = read().unwrap();
    }

    #[test]
    fn escape() {
        let mut song = Song::example();
        song.title = "title\t title 2".to_string();
        assert_ne!(song.serialize().into_bytes(), song.to_string().into_bytes());
    }
}

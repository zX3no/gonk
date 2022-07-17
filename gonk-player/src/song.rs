use std::{
    fs::File,
    path::{Path, PathBuf},
};
use symphonia::{
    core::{
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::{MetadataOptions, MetadataRevision, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

fn db_to_amplitude(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[derive(Debug, Clone, Default)]
pub struct Song {
    pub name: String,
    pub disc: u64,
    pub number: u64,
    pub path: PathBuf,
    pub gain: f32,
    pub album: String,
    pub artist: String,
    pub id: Option<usize>,
}

impl Song {
    pub fn from(path: &Path) -> Option<Song> {
        let file = Box::new(File::open(path).expect("Could not open file."));
        let mss = MediaSourceStream::new(file, MediaSourceStreamOptions::default());

        let mut probe = match get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(probe) => probe,
            Err(e) => {
                eprintln!("{}", e);
                eprintln!("{:?}", path);
                return None;
            }
        };

        let mut song = Song {
            name: String::from("Unknown Title"),
            disc: 1,
            number: 1,
            path: PathBuf::from(path.to_str().unwrap().replace('\\', "/")),
            gain: 0.0,
            album: String::from("Unknown Album"),
            artist: String::from("Unknown Artist"),
            id: None,
        };

        let mut update_metadata = |metadata: &MetadataRevision| {
            for tag in metadata.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        StandardTagKey::AlbumArtist => song.artist = tag.value.to_string(),
                        StandardTagKey::Artist if song.artist == "Unknown Artist" => {
                            song.artist = tag.value.to_string()
                        }
                        StandardTagKey::Album => song.album = tag.value.to_string(),
                        StandardTagKey::TrackTitle => song.name = tag.value.to_string(),
                        StandardTagKey::TrackNumber => {
                            let number = tag.value.to_string();
                            if let Some((num, _)) = number.split_once('/') {
                                song.number = num.parse().unwrap_or(1);
                            } else {
                                song.number = number.parse().unwrap_or(1);
                            }
                        }
                        StandardTagKey::DiscNumber => {
                            let number = tag.value.to_string();
                            if let Some((num, _)) = number.split_once('/') {
                                song.disc = num.parse().unwrap_or(1);
                            } else {
                                song.disc = number.parse().unwrap_or(1);
                            }
                        }
                        StandardTagKey::ReplayGainTrackGain => {
                            let db = tag
                                .value
                                .to_string()
                                .split(' ')
                                .next()
                                .unwrap()
                                .parse()
                                .unwrap_or(0.0);
                            song.gain = db_to_amplitude(db);
                        }
                        _ => (),
                    }
                }
            }
        };

        //Why are there two different ways to get metadata?
        if let Some(metadata) = probe.metadata.get() {
            if let Some(current) = metadata.current() {
                update_metadata(current);
            }
        } else if let Some(metadata) = probe.format.metadata().current() {
            update_metadata(metadata);
        }

        Some(song)
    }
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

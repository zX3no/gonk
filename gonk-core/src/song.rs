use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
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

fn db_to_amplitude(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0_f64)
}

#[derive(Debug, Clone, Default)]
pub struct Song {
    pub number: u64,
    pub disc: u64,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub path: PathBuf,
    pub duration: Duration,
    pub track_gain: f64,
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
            path: path.to_path_buf(),
            ..Default::default()
        };

        let mut get_songs = |metadata: &MetadataRevision| {
            for tag in metadata.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        StandardTagKey::AlbumArtist => song.artist = tag.value.to_string(),
                        StandardTagKey::Artist if song.artist.is_empty() => {
                            song.artist = tag.value.to_string();
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
                            song.track_gain = db_to_amplitude(db);
                        }
                        _ => (),
                    }
                }
            }
        };

        //TODO: Why are there two different ways to get metadata
        if let Some(metadata) = probe.metadata.get() {
            get_songs(metadata.current().unwrap());
        } else if let Some(metadata) = probe.format.metadata().current() {
            get_songs(metadata);
        }

        if song.artist.is_empty() {
            song.artist = String::from("Unknown Artist");
        }
        if song.name.is_empty() {
            song.name = String::from("Unknown Title");
        }
        if song.album.is_empty() {
            song.album = String::from("Unknown Album");
        }

        //Calculate duration
        let track = probe.format.default_track().unwrap();
        if let Some(tb) = track.codec_params.time_base {
            let ts = track.codec_params.start_ts;

            let dur = track
                .codec_params
                .n_frames
                .map(|frames| track.codec_params.start_ts + frames);

            if let Some(dur) = dur {
                let d = tb.calc_time(dur.saturating_sub(ts));
                let duration = Duration::from_secs(d.seconds) + Duration::from_secs_f64(d.frac);
                song.duration = duration;
            } else {
                song.duration = Duration::from_secs(0);
            }
        } else {
            song.duration = Duration::from_secs(0);
        }

        Some(song)
    }
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

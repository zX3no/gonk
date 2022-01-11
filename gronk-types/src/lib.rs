use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey},
    probe::Hint,
};

#[derive(Debug, Clone, Default)]
pub struct Song {
    pub number: u16,
    pub disc: u16,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub path: PathBuf,
    pub duration: Duration,
}
impl Song {
    pub fn from(path: &Path) -> Song {
        let mut hint = Hint::new();
        let ext = path.extension().unwrap().to_str().unwrap();
        hint.with_extension(ext);

        let file = Box::new(File::open(path).unwrap());

        // Create the media source stream using the boxed media source from above.
        let mss = MediaSourceStream::new(file, Default::default());

        // Use the default options for metadata and format readers.
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();

        let mut probe = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .unwrap();

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
                            song.artist = tag.value.to_string()
                        }
                        StandardTagKey::Album => song.album = tag.value.to_string(),
                        StandardTagKey::TrackTitle => song.name = tag.value.to_string(),
                        StandardTagKey::TrackNumber => {
                            let number = tag.value.to_string();
                            if let Some((num, _)) = number.split_once('/') {
                                song.number = num.parse::<u16>().unwrap_or(1);
                            } else {
                                song.number = tag.value.to_string().parse::<u16>().unwrap_or(1);
                            }
                        }
                        StandardTagKey::DiscNumber => {
                            let number = tag.value.to_string();
                            if let Some((num, _)) = number.split_once('/') {
                                song.disc = num.parse::<u16>().unwrap_or(1);
                            } else {
                                song.disc = tag.value.to_string().parse::<u16>().unwrap_or(1);
                            }
                        }
                        _ => (),
                    }
                }
            }
        };

        //TODO: why are there two different ways to get metadata
        if let Some(metadata) = probe.metadata.get() {
            get_songs(metadata.current().unwrap());
        } else if let Some(metadata) = probe.format.metadata().current() {
            get_songs(metadata);
        }

        if song.artist.is_empty() {
            song.artist = String::from("Unknown Artist");
        }

        //duration
        let track = probe.format.default_track().unwrap();
        let tb = track.codec_params.time_base.unwrap();
        let ts = track.codec_params.start_ts;

        let dur = track
            .codec_params
            .n_frames
            .map(|frames| track.codec_params.start_ts + frames)
            .unwrap();

        let d = tb.calc_time(dur.saturating_sub(ts));
        let duration = Duration::from_secs(d.seconds) + Duration::from_secs_f64(d.frac);
        song.duration = duration;

        song
    }
}
impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
            && self.disc == other.disc
            && self.name == other.name
            && self.album == other.album
            && self.artist == other.artist
            && self.path == other.path
            && self.duration == other.duration
    }
}

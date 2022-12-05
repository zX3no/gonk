use crate::{flac_decoder::*, Song, DISC_POS, GAIN_POS, NUMBER_POS};
use crate::{log, profile, SONG_LEN, TEXT_LEN};
use std::{fmt::Debug, fs::File, mem::size_of, path::Path, str::from_utf8_unchecked};
use symphonia::{
    core::{
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::{Limit, MetadataOptions, MetadataRevision, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

pub struct RawSong {
    pub text: [u8; TEXT_LEN],
    pub number: u8,
    pub disc: u8,
    pub gain: f32,
}

impl RawSong {
    pub fn new(
        artist: &str,
        album: &str,
        title: &str,
        path: &str,
        number: u8,
        disc: u8,
        gain: f32,
    ) -> Self {
        if path.len() > TEXT_LEN {
            panic!("PATH IS TOO LONG! {path}")
        }

        let mut artist = artist.to_string();
        let mut album = album.to_string();
        let mut title = title.to_string();

        //Forcefully fit the artist, album, title and path into 522 bytes.
        //There are 4 u16s included in the text so those are subtracted too.
        let mut i = 0;
        while artist.len() + album.len() + title.len() + path.len()
            > TEXT_LEN - (4 * size_of::<u16>())
        {
            if i % 3 == 0 {
                artist.pop();
            } else if i % 3 == 1 {
                album.pop();
            } else {
                title.pop();
            }
            i += 1;
        }

        if i != 0 {
            log!(
                "Warning: {} overflowed {} bytes! Metadata will be truncated.",
                path,
                SONG_LEN
            );
        }

        let mut text = [0; TEXT_LEN];

        let artist_len = (artist.len() as u16).to_le_bytes();
        text[0..2].copy_from_slice(&artist_len);
        text[2..2 + artist.len()].copy_from_slice(artist.as_bytes());

        let album_len = (album.len() as u16).to_le_bytes();
        text[2 + artist.len()..2 + artist.len() + 2].copy_from_slice(&album_len);
        text[2 + artist.len() + 2..2 + artist.len() + 2 + album.len()]
            .copy_from_slice(album.as_bytes());

        let title_len = (title.len() as u16).to_le_bytes();
        text[2 + artist.len() + 2 + album.len()..2 + artist.len() + 2 + album.len() + 2]
            .copy_from_slice(&title_len);
        text[2 + artist.len() + 2 + album.len() + 2
            ..2 + artist.len() + 2 + album.len() + 2 + title.len()]
            .copy_from_slice(title.as_bytes());

        let path_len = (path.len() as u16).to_le_bytes();
        text[2 + artist.len() + 2 + album.len() + 2 + title.len()
            ..2 + artist.len() + 2 + album.len() + 2 + title.len() + 2]
            .copy_from_slice(&path_len);
        text[2 + artist.len() + 2 + album.len() + 2 + title.len() + 2
            ..2 + artist.len() + 2 + album.len() + 2 + title.len() + 2 + path.len()]
            .copy_from_slice(path.as_bytes());

        Self {
            text,
            number,
            disc,
            gain,
        }
    }
    pub fn as_bytes(&self) -> [u8; SONG_LEN] {
        let mut song = [0u8; SONG_LEN];
        assert!(self.text.len() <= TEXT_LEN);

        song[..self.text.len()].copy_from_slice(&self.text);
        song[NUMBER_POS] = self.number;
        song[DISC_POS] = self.disc;
        song[GAIN_POS].copy_from_slice(&self.gain.to_le_bytes());
        song
    }
    pub fn artist(&self) -> &str {
        artist(&self.text)
    }
    pub fn album(&self) -> &str {
        album(&self.text)
    }
    pub fn title(&self) -> &str {
        title(&self.text)
    }
    pub fn path(&self) -> &str {
        path(&self.text)
    }
    pub fn from_path(path: &'_ Path) -> Result<RawSong, String> {
        let ex = path.extension().unwrap();
        if ex == "flac" {
            profile!("custom::decode");
            match read_metadata(path) {
                Ok(metadata) => {
                    let number = metadata
                        .get("TRACKNUMBER")
                        .unwrap_or(&String::from("1"))
                        .parse()
                        .unwrap_or(1);
                    let disc = metadata
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

                    Ok(RawSong::new(
                        #[allow(clippy::or_fun_call)]
                        metadata.get("ALBUMARTIST").unwrap_or(
                            metadata
                                .get("ARTIST")
                                .unwrap_or(&String::from("Unknown Artist")),
                        ),
                        metadata
                            .get("ALBUM")
                            .unwrap_or(&String::from("Unknown Album")),
                        metadata
                            .get("TITLE")
                            .unwrap_or(&String::from("Unknown Title")),
                        &path.to_string_lossy(),
                        number,
                        disc,
                        gain,
                    ))
                }
                Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
            }
        } else {
            profile!("symphonia::decode");
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
                Err(err) => return Err(format!("Error: ({err}) @ {}", path.to_string_lossy())),
            };

            let mut title = String::from("Unknown Title");
            let mut album = String::from("Unknown Album");
            let mut artist = String::from("Unknown Artist");
            let mut number = 1;
            let mut disc = 1;
            let mut gain = 0.0;

            let mut update_metadata = |metadata: &MetadataRevision| {
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
                                    number = num.parse().unwrap_or(1);
                                } else {
                                    number = num.parse().unwrap_or(1);
                                }
                            }
                            StandardTagKey::DiscNumber => {
                                let num = tag.value.to_string();
                                if let Some((num, _)) = num.split_once('/') {
                                    disc = num.parse().unwrap_or(1);
                                } else {
                                    disc = num.parse().unwrap_or(1);
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

                                gain = 10.0f32.powf(db / 20.0);
                            }
                            _ => (),
                        }
                    }
                }
            };

            if let Some(metadata) = probe.format.metadata().skip_to_latest() {
                update_metadata(metadata);
            } else if let Some(mut metadata) = probe.metadata.get() {
                let metadata = metadata.skip_to_latest().unwrap();
                update_metadata(metadata);
            } else {
                //Probably a WAV file that doesn't have metadata.
            }

            Ok(RawSong::new(
                &artist,
                &album,
                &title,
                &path.to_string_lossy(),
                number,
                disc,
                gain,
            ))
        }
    }
}

impl Default for RawSong {
    fn default() -> Self {
        Self::new("artist", "album", "title", "path", 12, 1, 0.123)
    }
}

impl Debug for RawSong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let title = title(&self.text);
        let album = album(&self.text);
        let artist = artist(&self.text);
        let path = path(&self.text);
        f.debug_struct("Song")
            .field("artist", &artist)
            .field("album", &album)
            .field("title", &title)
            .field("path", &path)
            .field("number", &self.number)
            .field("disc", &self.disc)
            .field("gain", &self.gain)
            .finish()
    }
}

impl From<&'_ [u8]> for RawSong {
    fn from(bytes: &[u8]) -> Self {
        Self {
            text: bytes[..TEXT_LEN].try_into().unwrap(),
            number: bytes[NUMBER_POS],
            disc: bytes[DISC_POS],
            gain: f32::from_le_bytes(bytes[GAIN_POS].try_into().unwrap()),
        }
    }
}

impl From<&Song> for RawSong {
    fn from(song: &Song) -> Self {
        RawSong::new(
            &song.artist,
            &song.album,
            &song.title,
            &song.path,
            song.track_number,
            song.disc_number,
            song.gain,
        )
    }
}

pub const fn artist_len(text: &[u8]) -> u16 {
    u16::from_le_bytes([text[0], text[1]])
}

pub const fn album_len(text: &[u8], artist_len: usize) -> u16 {
    u16::from_le_bytes([text[2 + artist_len], text[2 + artist_len + 1]])
}

pub const fn title_len(text: &[u8], artist_len: usize, album_len: usize) -> u16 {
    u16::from_le_bytes([
        text[2 + artist_len + 2 + album_len],
        text[2 + artist_len + 2 + album_len + 1],
    ])
}

pub const fn path_len(text: &[u8], artist_len: usize, album_len: usize, title_len: usize) -> u16 {
    u16::from_le_bytes([
        text[2 + artist_len + 2 + album_len + 2 + title_len],
        text[2 + artist_len + 2 + album_len + 2 + title_len + 1],
    ])
}

pub const fn artist(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;

    unsafe {
        let slice = text.get_unchecked(2..artist_len + 2);
        from_utf8_unchecked(slice)
    }
}

pub const fn album(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;

    unsafe {
        let slice = text.get_unchecked(2 + artist_len + 2..2 + artist_len + 2 + album_len);
        from_utf8_unchecked(slice)
    }
}

pub const fn title(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    let title_len = title_len(text, artist_len, album_len) as usize;

    unsafe {
        let slice = text.get_unchecked(
            2 + artist_len + 2 + album_len + 2..2 + artist_len + 2 + album_len + 2 + title_len,
        );
        from_utf8_unchecked(slice)
    }
}

pub const fn path(text: &[u8]) -> &str {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    let title_len = title_len(text, artist_len, album_len) as usize;
    let path_len = path_len(text, artist_len, album_len, title_len) as usize;
    unsafe {
        let slice = text.get_unchecked(
            2 + artist_len + 2 + album_len + 2 + title_len + 2
                ..2 + artist_len + 2 + album_len + 2 + title_len + 2 + path_len,
        );
        from_utf8_unchecked(slice)
    }
}

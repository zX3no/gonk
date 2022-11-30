#![feature(test)]
#![feature(const_slice_index)]
#![feature(const_float_bits_conv)]
#![allow(clippy::missing_safety_doc)]

use flac_decoder::*;
use std::{
    env,
    error::Error,
    fmt::Debug,
    fs::{self, File},
    io::{self, BufWriter, Write},
    mem::size_of,
    ops::Range,
    path::{Path, PathBuf},
    str::{from_utf8, from_utf8_unchecked},
    thread::{self},
    time::Instant,
};
use symphonia::{
    core::{
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::{Limit, MetadataOptions, MetadataRevision, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};

//TODO: Could this be stored in a more drive friendly size like 512?
//522 + 1 + 1 + 4 = 528

//TODO: Surely there is a better way of doing this...
pub const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() + size_of::<u8>() + size_of::<f32>();
pub const TEXT_LEN: usize = 522;

//522
pub const NUMBER_POS: usize = SONG_LEN - 1 - size_of::<f32>() - size_of::<u8>();

//523
pub const DISC_POS: usize = SONG_LEN - 1 - size_of::<f32>();

//524, 525, 526, 527
pub const GAIN_POS: Range<usize> = SONG_LEN - size_of::<f32>()..SONG_LEN;

pub mod db;
pub use db::{Database, Song};

mod flac_decoder;
mod index;
mod playlist;

pub mod data;
pub mod log;
pub mod profiler;

pub use index::*;
pub use playlist::*;

pub static mut SETTINGS: Settings = Settings::default();

pub fn gonk_path() -> PathBuf {
    let gonk = if cfg!(windows) {
        PathBuf::from(&env::var("APPDATA").unwrap())
    } else {
        PathBuf::from(&env::var("HOME").unwrap()).join(".config")
    }
    .join("gonk");

    if !gonk.exists() {
        fs::create_dir_all(&gonk).unwrap();
    }

    gonk
}

pub fn settings_path() -> PathBuf {
    let mut path = database_path();
    path.pop();
    path.push("settings.db");
    path
}

pub fn database_path() -> PathBuf {
    let gonk = gonk_path();

    //Backwards compatibility for older versions of gonk
    let old_db = gonk.join("gonk_new.db");
    let db = gonk.join("gonk.db");

    if old_db.exists() {
        fs::rename(old_db, &db).unwrap();
    }

    db
}

//Delete the settings and overwrite with updated values.
pub fn save_settings() {
    //Opening a thread takes 47us
    //Opening the file takes 200us
    thread::spawn(|| {
        unsafe {
            //Opening the same file twice may cause your computer to explode.
            let file = File::options()
                .write(true)
                .truncate(true)
                .create(true)
                .open(settings_path())
                .unwrap();

            let writer = BufWriter::new(&file);
            SETTINGS.write(writer).unwrap();
        };
    });
}

fn validate(file: &[u8]) -> Result<(), Box<dyn Error>> {
    if file.is_empty() {
        return Ok(());
    } else if file.len() < SONG_LEN {
        return Err("Invalid song")?;
    }

    let text = &file[..TEXT_LEN];

    let artist_len = artist_len(text) as usize;
    if artist_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let _artist = from_utf8(&text[2..artist_len + 2])?;

    let album_len = album_len(text, artist_len) as usize;
    if album_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let _album = from_utf8(&text[2 + artist_len + 2..artist_len + 2 + album_len + 2])?;

    let title_len = title_len(text, artist_len, album_len) as usize;
    if title_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let _title = from_utf8(
        &text[2 + artist_len + 2 + album_len + 2..artist_len + 2 + album_len + 2 + title_len + 2],
    )?;

    let path_len = path_len(text, artist_len, album_len, title_len) as usize;
    if path_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let _path = from_utf8(
        &text[2 + artist_len + 2 + album_len + 2 + title_len + 2
            ..artist_len + 2 + album_len + 2 + title_len + 2 + path_len + 2],
    )?;

    let _number = file[NUMBER_POS];
    let _disc = file[DISC_POS];
    let _gain = f32::from_le_bytes([
        file[SONG_LEN - 5],
        file[SONG_LEN - 4],
        file[SONG_LEN - 3],
        file[SONG_LEN - 2],
    ]);

    Ok(())
}

// Settings Layout:
//
// Volume
// Index
// u16 (output_device length)
// output_device
// u16 (music_folder length)
// music_folder
// [RawSong]
#[derive(Debug)]
pub struct Settings {
    pub volume: u8,
    pub index: u16,
    pub elapsed: f32,
    pub output_device: String,
    pub music_folder: String,
    pub queue: Vec<RawSong>,
}

impl Settings {
    pub const fn default() -> Self {
        Self {
            volume: 15,
            index: 0,
            elapsed: 0.0,
            output_device: String::new(),
            music_folder: String::new(),
            queue: Vec::new(),
        }
    }
    pub fn from(bytes: Vec<u8>) -> Option<Self> {
        unsafe {
            let volume = bytes[0];
            let index = u16::from_le_bytes([bytes[1], bytes[2]]);
            let elapsed = f32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);

            let output_device_len = u16::from_le_bytes([bytes[7], bytes[8]]) as usize + 9;
            if output_device_len >= bytes.len() {
                return None;
            }
            let output_device = from_utf8_unchecked(&bytes[9..output_device_len]).to_string();

            let start = output_device_len + size_of::<u16>();
            let music_folder_len =
                u16::from_le_bytes([bytes[output_device_len], bytes[output_device_len + 1]])
                    as usize;
            if music_folder_len >= bytes.len() {
                return None;
            }
            let music_folder =
                from_utf8_unchecked(&bytes[start..start + music_folder_len]).to_string();

            let mut queue = Vec::new();
            let mut i = start + music_folder_len;
            while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
                queue.push(RawSong::from(bytes));
                i += SONG_LEN;
            }

            Some(Self {
                index,
                volume,
                output_device,
                music_folder,
                elapsed,
                queue,
            })
        }
    }
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.volume);
        bytes.extend(self.index.to_le_bytes());
        bytes.extend(self.elapsed.to_le_bytes());

        bytes.extend((self.output_device.len() as u16).to_le_bytes());
        bytes.extend(self.output_device.as_bytes());

        bytes.extend((self.music_folder.len() as u16).to_le_bytes());
        bytes.extend(self.music_folder.as_bytes());

        for song in &self.queue {
            bytes.extend(song.into_bytes());
        }
        bytes
    }
    pub fn write(&self, mut writer: BufWriter<&File>) -> io::Result<()> {
        writer.write_all(&[self.volume])?;
        writer.write_all(&self.index.to_le_bytes())?;
        writer.write_all(&self.elapsed.to_le_bytes())?;

        writer.write_all(&(self.output_device.len() as u16).to_le_bytes())?;
        writer.write_all(self.output_device.as_bytes())?;

        writer.write_all(&(self.music_folder.len() as u16).to_le_bytes())?;
        writer.write_all(self.music_folder.as_bytes())?;

        for song in &self.queue {
            writer.write_all(&song.into_bytes())?;
        }
        Ok(())
    }
}

pub fn save_volume(new_volume: u8) {
    unsafe {
        SETTINGS.volume = new_volume;
        save_settings();
    }
}

pub fn save_queue(queue: &[Song], index: u16, elapsed: f32) {
    unsafe {
        SETTINGS.queue = queue.iter().map(RawSong::from).collect();
        SETTINGS.index = index;
        SETTINGS.elapsed = elapsed;
        save_settings();
    };
}

pub fn update_queue_state(index: u16, elapsed: f32) {
    unsafe {
        SETTINGS.elapsed = elapsed;
        SETTINGS.index = index;
        save_settings();
    }
}

pub fn update_output_device(device: &str) {
    unsafe {
        SETTINGS.output_device = device.to_string();
        save_settings();
    }
}

pub fn update_music_folder(folder: &str) {
    unsafe {
        SETTINGS.music_folder = folder.replace('\\', "/");
        save_settings();
    }
}

pub fn get_queue() -> (Vec<Song>, Option<usize>, f32) {
    unsafe {
        let index = if SETTINGS.queue.is_empty() {
            None
        } else {
            Some(SETTINGS.index as usize)
        };

        (
            SETTINGS
                .queue
                .iter()
                .map(|song| Song::from(&song.into_bytes()))
                .collect(),
            index,
            SETTINGS.elapsed,
        )
    }
}

pub fn output_device() -> &'static str {
    unsafe { &SETTINGS.output_device }
}

pub fn music_folder() -> &'static str {
    unsafe { &SETTINGS.music_folder }
}

pub fn volume() -> u8 {
    unsafe { SETTINGS.volume }
}

pub enum ScanResult {
    Completed,
    CompletedWithErrors(Vec<String>),
    Incomplete(&'static str),
}

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
    pub fn into_bytes(&self) -> [u8; SONG_LEN] {
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

pub const fn artist_and_album(text: &[u8]) -> (&str, &str) {
    debug_assert!(text.len() == TEXT_LEN);
    let artist_len = artist_len(text) as usize;
    let album_len = album_len(text, artist_len) as usize;
    unsafe {
        let artist = text.get_unchecked(2..2 + artist_len);
        let album = text.get_unchecked(2 + artist_len + 2..2 + artist_len + 2 + album_len);
        (from_utf8_unchecked(artist), from_utf8_unchecked(album))
    }
}

pub fn bench<F>(func: F)
where
    F: Fn(),
{
    let now = Instant::now();
    for _ in 0..100_000 {
        func();
    }
    println!("{:?}", now.elapsed());
}

pub fn bench_slow<F>(func: F)
where
    F: Fn(),
{
    let now = Instant::now();
    for _ in 0..4000 {
        func();
    }
    println!("{:?}", now.elapsed());
}

pub fn bench_super_slow<F>(func: F)
where
    F: Fn(),
{
    let now = Instant::now();
    for _ in 0..500 {
        func();
    }
    println!("{:?}", now.elapsed());
}

#[cfg(test)]
mod tests {
    use crate::*;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator, ParallelSliceMut};
    extern crate test;
    use tempfile::tempfile;
    use test::Bencher;

    #[bench]
    fn bench_collect_all(b: &mut Bencher) {
        let file = tempfile().unwrap();

        let mut writer = BufWriter::new(&file);
        for i in 0..10_000 {
            let song = RawSong::new(
                &format!("{} artist", i),
                &format!("{} album", i),
                &format!("{} title", i),
                &format!("{} path", i),
                1,
                1,
                0.25,
            );
            writer.write_all(&song.into_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };

        b.iter(|| {
            let songs: Vec<Song> = (0..mmap.len() / SONG_LEN)
                .into_par_iter()
                .map(|i| {
                    let pos = i * SONG_LEN;
                    let bytes = &mmap[pos..pos + SONG_LEN];
                    Song::from(bytes)
                })
                .collect();
            assert_eq!(songs.len(), 10_000);

            let mut albums: Vec<(&str, &str)> = songs
                .iter()
                .map(|song| (song.artist.as_str(), song.album.as_str()))
                .collect();
            albums.par_sort_unstable_by_key(|(artist, _album)| artist.to_ascii_lowercase());
            albums.dedup();
            assert_eq!(albums.len(), 10_000);

            let albums: Vec<(String, String)> = albums
                .into_iter()
                .map(|(artist, album)| (artist.to_owned(), album.to_owned()))
                .collect();
            assert_eq!(albums.len(), 10_000);

            let mut artists: Vec<String> = albums
                .iter()
                .map(|(artist, _album)| artist.clone())
                .collect();
            artists.dedup();

            assert_eq!(artists.len(), 10_000);
        });
    }
    #[bench]
    fn bench_collect_artist_single(b: &mut Bencher) {
        let file = tempfile().unwrap();

        let mut writer = BufWriter::new(&file);
        for i in 0..10_000 {
            let song = RawSong::new(
                &format!("{} artist", i),
                &format!("{} album", i),
                &format!("{} title", i),
                &format!("{} path", i),
                1,
                1,
                0.25,
            );
            writer.write_all(&song.into_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };

        b.iter(|| {
            let mut songs = Vec::new();
            let mut i = 0;
            while let Some(text) = mmap.get(i..i + TEXT_LEN) {
                let artist = artist(text);
                if artist == "9999 artist" {
                    let song_bytes = &mmap[i..i + SONG_LEN];
                    songs.push(Song::from(song_bytes));
                }
                i += SONG_LEN;
            }
            assert_eq!(songs.len(), 1);
        });
    }
    #[bench]
    fn bench_collect_artist(b: &mut Bencher) {
        let file = tempfile().unwrap();

        let mut writer = BufWriter::new(&file);
        let song = RawSong::new("artist", "album", "title", "path", 1, 1, 0.25);
        for _ in 0..10_000 {
            writer.write_all(&song.into_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };

        b.iter(|| {
            let mut songs = Vec::new();
            let mut i = 0;
            while let Some(text) = mmap.get(i..i + TEXT_LEN) {
                let artist = artist(text);
                if artist == "artist" {
                    let song_bytes = &mmap[i..i + SONG_LEN];
                    songs.push(Song::from(song_bytes));
                }
                i += SONG_LEN;
            }
            assert_eq!(songs.len(), 10000);
        });
    }
    #[test]
    fn clamp_song() {
        let song = RawSong::new(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1,
            1,
            0.25,
        );
        assert_eq!(song.artist().len(), 126);
        assert_eq!(song.album().len(), 127);
        assert_eq!(song.title().len(), 127);
        assert_eq!(song.path().len(), 134);
        assert_eq!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".len(), 134);
    }

    #[test]
    fn settings() {
        let mut settings = Settings::default();
        let song = RawSong::new("artist", "album", "title", "path", 1, 1, 0.25);
        settings.queue.push(song);

        let bytes = settings.as_bytes();
        let new_settings = Settings::from(bytes).unwrap();

        assert_eq!(settings.volume, new_settings.volume);
        assert_eq!(settings.index, new_settings.index);
        assert_eq!(settings.elapsed, new_settings.elapsed);
        assert_eq!(settings.output_device, new_settings.output_device);
        assert_eq!(settings.music_folder, new_settings.music_folder);
    }

    #[test]
    fn database() {
        let mut db = Vec::new();
        for i in 0..10_000 {
            let song = RawSong::new(
                &format!("{} artist", i),
                &format!("{} album", i),
                &format!("{} title", i),
                &format!("{} path", i),
                1,
                1,
                0.25,
            );
            db.extend(song.into_bytes());
        }

        assert_eq!(db.len(), 5280000);
        assert_eq!(db.len() / SONG_LEN, 10_000);
        assert_eq!(artist(&db[..TEXT_LEN]), "0 artist");
        assert_eq!(album(&db[..TEXT_LEN]), "0 album");
        assert_eq!(title(&db[..TEXT_LEN]), "0 title");
        assert_eq!(path(&db[..TEXT_LEN]), "0 path");
        assert_eq!(artist_and_album(&db[..TEXT_LEN]), ("0 artist", "0 album"));

        assert_eq!(
            artist(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 artist"
        );
        assert_eq!(
            album(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 album"
        );
        assert_eq!(
            title(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 title"
        );
        assert_eq!(
            path(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            "1000 path"
        );
        assert_eq!(
            artist_and_album(&db[SONG_LEN * 1000..SONG_LEN * 1001 - (SONG_LEN - TEXT_LEN)]),
            ("1000 artist", "1000 album")
        );

        let song = Song::from(&db[..SONG_LEN]);
        assert_eq!(song.artist, "0 artist");
        assert_eq!(song.album, "0 album");
        assert_eq!(song.title, "0 title");
        assert_eq!(song.path, "0 path");
        assert_eq!(song.track_number, 1);
        assert_eq!(song.disc_number, 1);
        assert_eq!(song.gain, 0.25);

        let song = Song::from(&db[SONG_LEN * 9999..SONG_LEN * 10000]);
        assert_eq!(song.artist, "9999 artist");
        assert_eq!(song.album, "9999 album");
        assert_eq!(song.title, "9999 title");
        assert_eq!(song.path, "9999 path");
        assert_eq!(song.track_number, 1);
        assert_eq!(song.disc_number, 1);
        assert_eq!(song.gain, 0.25);
    }
}

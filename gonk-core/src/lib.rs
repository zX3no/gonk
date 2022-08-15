use arrayvec::ArrayVec;
use memmap2::Mmap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    env,
    error::Error,
    fmt::Debug,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    mem::size_of,
    ops::Range,
    path::{Path, PathBuf},
    str::{from_utf8, from_utf8_unchecked},
    thread::{self, JoinHandle},
    time::Instant,
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
use walkdir::{DirEntry, WalkDir};

//522 + 1 + 1 + 4
pub const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() + size_of::<u8>() + size_of::<f32>();
pub const TEXT_LEN: usize = 522;

pub const NUMBER_POS: usize = SONG_LEN - 1 - 4 - 2;
pub const DISC_POS: usize = SONG_LEN - 1 - 4 - 1;
pub const GAIN_POS: Range<usize> = SONG_LEN - 1 - 4..SONG_LEN - 1;

mod index;
mod playlist;
mod query;

pub mod log;

pub use index::*;
pub use playlist::*;
pub use query::*;

pub static mut MMAP: Option<Mmap> = None;
pub static mut SETTINGS: Settings = Settings::default();

thread_local! {
    pub static SETTINGS_FILE: File = {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(settings_path())
            .unwrap()
    };
}

pub fn settings_path() -> PathBuf {
    let mut path = database_path();
    path.pop();
    path.push("settings.db");
    path
}

pub fn database_path() -> PathBuf {
    let gonk = if cfg!(windows) {
        PathBuf::from(&env::var("APPDATA").unwrap())
    } else {
        PathBuf::from(&env::var("HOME").unwrap()).join(".config")
    }
    .join("gonk");

    if !gonk.exists() {
        fs::create_dir_all(&gonk).unwrap();
    }

    //Backwards compatibility for older versions of gonk
    let old_db = gonk.join("gonk_new.db");
    let db = gonk.join("gonk.db");

    if old_db.exists() {
        old_db
    } else {
        db
    }
}

pub fn init() {
    SETTINGS_FILE.with(|mut file| {
        let mut buffer = Vec::new();
        match file.read_to_end(&mut buffer) {
            Ok(_) if !buffer.is_empty() => unsafe { SETTINGS = Settings::from(buffer) },
            Err(_) => unreachable!("Failed to load settings"),
            _ => {
                //Save the default settings if nothing is found.
                save_settings();
            }
        }
    });

    let db = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&database_path())
        .unwrap();

    unsafe { MMAP = Some(Mmap::map(&db).unwrap()) };

    //Reset the database if the first song is invalid.
    if validate().is_err() {
        log!("Database is corrupted. Resetting!");
        reset().unwrap();
    }
}

//Delete the settings and overwrite with updated values.
pub fn save_settings() {
    SETTINGS_FILE.with(|file| {
        let writer = BufWriter::new(file);
        unsafe { SETTINGS.write(writer).unwrap() };
    });
}

pub fn reset() -> io::Result<()> {
    if let Some(mmap) = unsafe { MMAP.take() } {
        drop(mmap);
    }

    unsafe { SETTINGS = Settings::default() };
    fs::remove_file(settings_path())?;
    fs::remove_file(database_path())
}

fn validate() -> Result<(), Box<dyn Error>> {
    let mmap = mmap().unwrap();

    if mmap.is_empty() {
        return Ok(());
    } else if mmap.len() < SONG_LEN {
        return Err("Invalid song")?;
    }
    let text = &mmap[..TEXT_LEN];
    let artist_len = u16::from_le_bytes(text[0..2].try_into()?) as usize;
    if artist_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let _artist = from_utf8(&text[2..artist_len + 2])?;

    let album_len =
        u16::from_le_bytes(text[2 + artist_len..2 + artist_len + 2].try_into()?) as usize;
    if album_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let album = 2 + artist_len + 2..artist_len + 2 + album_len + 2;
    let _album = from_utf8(&text[album])?;

    let title_len = u16::from_le_bytes(
        text[2 + artist_len + 2 + album_len..2 + artist_len + 2 + album_len + 2].try_into()?,
    ) as usize;
    if title_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let title = 2 + artist_len + 2 + album_len + 2..artist_len + 2 + album_len + 2 + title_len + 2;
    let _title = from_utf8(&text[title])?;

    let path_len = u16::from_le_bytes(
        text[2 + artist_len + 2 + album_len + 2 + title_len
            ..2 + artist_len + 2 + album_len + 2 + title_len + 2]
            .try_into()?,
    ) as usize;
    if path_len > TEXT_LEN {
        Err("Invalid u16")?;
    }
    let path = 2 + artist_len + 2 + album_len + 2 + title_len + 2
        ..artist_len + 2 + album_len + 2 + title_len + 2 + path_len + 2;
    let _path = from_utf8(&text[path])?;

    let _number = mmap[NUMBER_POS];
    let _disc = mmap[DISC_POS];
    let _gain = f32::from_le_bytes(mmap[GAIN_POS].try_into()?);

    Ok(())
}

//TODO: Remove null terminated strings
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
    pub fn from(bytes: Vec<u8>) -> Self {
        unsafe {
            let volume = bytes[0];
            let index = u16::from_le_bytes(bytes[1..3].try_into().unwrap());
            let elapsed = f32::from_le_bytes(bytes[3..7].try_into().unwrap());
            let end = bytes[7..].iter().position(|&c| c == b'\0').unwrap() + 7;
            let output_device = from_utf8_unchecked(&bytes[7..end]).to_string();
            let old_end = end + 1;
            let end = bytes[old_end..].iter().position(|&c| c == b'\0').unwrap() + old_end;
            let music_folder = from_utf8_unchecked(&bytes[old_end..end]).to_string();

            let mut queue = Vec::new();
            //Skip the null terminator
            let mut i = end + 1;
            while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
                queue.push(RawSong::from(bytes));
                i += SONG_LEN;
            }

            Self {
                index,
                volume,
                output_device,
                music_folder,
                elapsed,
                queue,
            }
        }
    }
    pub fn write(&self, mut writer: BufWriter<&File>) -> io::Result<()> {
        writer.write_all(&[self.volume])?;
        writer.write_all(&self.index.to_le_bytes())?;
        writer.write_all(&self.elapsed.to_le_bytes())?;
        writer.write_all(self.output_device.replace('\0', "").as_bytes())?;
        writer.write_all(&[b'\0'])?;
        writer.write_all(self.music_folder.replace('\0', "").as_bytes())?;
        writer.write_all(&[b'\0'])?;
        for song in &self.queue {
            writer.write_all(&song.into_bytes())?;
        }
        Ok(())
    }
}

pub fn update_volume(new_volume: u8) {
    unsafe {
        SETTINGS.volume = new_volume;
        save_settings();
    }
}

//You know it's bad you need to spawn a new thread.
//What if just the index was updated? Why do you need to write everything again.
pub fn update_queue(queue: &[Song], index: u16, elapsed: f32) {
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
                .map(|song| Song::from(&song.into_bytes(), 0))
                .collect(),
            index,
            SETTINGS.elapsed,
        )
    }
}

pub fn get_output_device() -> &'static str {
    unsafe { &SETTINGS.output_device }
}

pub fn get_music_folder() -> &'static str {
    unsafe { &SETTINGS.music_folder }
}

pub fn volume() -> u8 {
    unsafe { SETTINGS.volume }
}

pub fn mmap() -> Option<&'static Mmap> {
    unsafe { MMAP.as_ref() }
}

pub fn scan(path: String) -> JoinHandle<()> {
    unsafe {
        let mmap = MMAP.take().unwrap();
        drop(mmap);
        debug_assert!(MMAP.is_none());
    }

    thread::spawn(|| {
        match OpenOptions::new()
            .write(true)
            .read(true)
            .truncate(true)
            .open(&database_path())
        {
            Ok(file) => {
                let mut writer = BufWriter::new(&file);

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

                let songs: Vec<RawSong> = paths
                    .into_par_iter()
                    .map(|path| RawSong::from(path.path()))
                    .collect();

                for song in songs {
                    writer.write_all(&song.into_bytes()).unwrap();
                }

                writer.flush().unwrap();
                unsafe { MMAP = Some(Mmap::map(&file).unwrap()) };
            }
            Err(_) => log!("Failed to scan folder, database is already open."),
        }
    })
}

#[derive(Clone, Debug)]
pub struct Song {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub path: String,
    pub number: u8,
    pub disc: u8,
    pub gain: f32,
    pub id: usize,
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.artist == other.artist
            && self.album == other.album
            && self.title == other.title
            && self.path == other.path
            && self.number == other.number
            && self.disc == other.disc
            && self.gain == other.gain
            && self.id == other.id
    }
}

impl PartialOrd for Song {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.artist == other.artist {
            if self.album == other.album {
                if self.disc == other.disc {
                    self.number.partial_cmp(&other.number)
                } else {
                    self.disc.partial_cmp(&other.disc)
                }
            } else {
                self.album.partial_cmp(&other.album)
            }
        } else {
            self.artist.partial_cmp(&other.artist)
        }
    }
}

impl Eq for Song {}

impl Ord for Song {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Song {
    pub fn from(bytes: &[u8], id: usize) -> Self {
        unsafe {
            let text = &bytes[..TEXT_LEN];
            let artist_len = u16::from_le_bytes(text[0..2].try_into().unwrap()) as usize;
            let artist = from_utf8_unchecked(&text[2..artist_len + 2]);

            let album_len =
                u16::from_le_bytes(text[2 + artist_len..2 + artist_len + 2].try_into().unwrap())
                    as usize;
            let album = 2 + artist_len + 2..artist_len + 2 + album_len + 2;
            let album = from_utf8_unchecked(&text[album]);

            let title_len = u16::from_le_bytes(
                text[2 + artist_len + 2 + album_len..2 + artist_len + 2 + album_len + 2]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let title =
                2 + artist_len + 2 + album_len + 2..artist_len + 2 + album_len + 2 + title_len + 2;
            let title = from_utf8_unchecked(&text[title]);

            let path_len = u16::from_le_bytes(
                text[2 + artist_len + 2 + album_len + 2 + title_len
                    ..2 + artist_len + 2 + album_len + 2 + title_len + 2]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let path = 2 + artist_len + 2 + album_len + 2 + title_len + 2
                ..artist_len + 2 + album_len + 2 + title_len + 2 + path_len + 2;
            let path = from_utf8_unchecked(&text[path]);

            let number = bytes[NUMBER_POS];
            let disc = bytes[DISC_POS];
            let gain = f32::from_le_bytes(bytes[GAIN_POS].try_into().unwrap());

            Self {
                artist: artist.to_string(),
                album: album.to_string(),
                title: title.to_string(),
                path: path.to_string(),
                number,
                disc,
                gain,
                id,
            }
        }
    }
}

pub struct RawSong {
    pub text: ArrayVec<u8, TEXT_LEN>,
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
            panic!("PATH IS TOO LONG! {}", path)
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

        let mut text = ArrayVec::<u8, TEXT_LEN>::new();

        text.extend((artist.len() as u16).to_le_bytes());
        let _ = text.try_extend_from_slice(artist.as_bytes());
        text.extend((album.len() as u16).to_le_bytes());
        let _ = text.try_extend_from_slice(album.as_bytes());
        text.extend((title.len() as u16).to_le_bytes());
        let _ = text.try_extend_from_slice(title.as_bytes());
        text.extend((path.len() as u16).to_le_bytes());
        let _ = text.try_extend_from_slice(path.as_bytes());

        Self {
            text,
            number,
            disc,
            gain,
        }
    }
    pub fn into_bytes(&self) -> [u8; SONG_LEN] {
        let mut song = [0u8; SONG_LEN];
        debug_assert!(self.text.len() <= SONG_LEN);

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
            song.number,
            song.disc,
            song.gain,
        )
    }
}

impl From<&'_ Path> for RawSong {
    fn from(path: &'_ Path) -> Self {
        let file = Box::new(File::open(path).expect("Could not open file."));
        let mss = MediaSourceStream::new(file, MediaSourceStreamOptions::default());

        let mut probe = match get_probe().format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(probe) => probe,
            Err(_) => panic!("{:?}", path),
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

        RawSong::new(
            &artist,
            &album,
            &title,
            &path.to_string_lossy(),
            number,
            disc,
            gain,
        )
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

        let song = Song::from(&db[..SONG_LEN], 0);
        assert_eq!(song.artist, "0 artist");
        assert_eq!(song.album, "0 album");
        assert_eq!(song.title, "0 title");
        assert_eq!(song.path, "0 path");
        assert_eq!(song.number, 1);
        assert_eq!(song.disc, 1);
        assert_eq!(song.gain, 0.25);

        let song = Song::from(&db[SONG_LEN * 9999..SONG_LEN * 10000], 9999);
        assert_eq!(song.artist, "9999 artist");
        assert_eq!(song.album, "9999 album");
        assert_eq!(song.title, "9999 title");
        assert_eq!(song.path, "9999 path");
        assert_eq!(song.number, 1);
        assert_eq!(song.disc, 1);
        assert_eq!(song.gain, 0.25);
    }
}

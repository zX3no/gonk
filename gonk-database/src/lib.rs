use memmap2::Mmap;
use rayon::{
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{
    env,
    fmt::Debug,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    mem::size_of,
    path::{Path, PathBuf},
    str::from_utf8_unchecked,
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
use walkdir::DirEntry;

const TEXT_LEN: usize = 510;
const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() * 2;

pub fn name(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    unsafe {
        let end = text.iter().position(|&c| c == b'\0').unwrap_unchecked();
        from_utf8_unchecked(&text[..end])
    }
}

pub fn album(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut start = 0;

    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            if start == 0 {
                start = i + 1;
            } else {
                return unsafe { from_utf8_unchecked(&text[start..i]) };
            }
        }
    }
    unreachable!();
}

pub fn artist(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut pos = [None; 2];
    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            if pos[0].is_none() {
                pos[0] = Some(i);
            } else if pos[1].is_none() {
                pos[1] = Some(i);
            } else {
                return unsafe { from_utf8_unchecked(&text[pos[1].unwrap_unchecked() + 1..i]) };
            }
        }
    }
    unreachable!();
}

pub fn path(text: &[u8]) -> &str {
    debug_assert_eq!(text.len(), TEXT_LEN);
    let mut pos = [None; 3];
    for (i, c) in text.iter().enumerate() {
        if c == &b'\0' {
            if pos[0].is_none() {
                pos[0] = Some(i);
            } else if pos[1].is_none() {
                pos[1] = Some(i);
            } else if pos[2].is_none() {
                pos[2] = Some(i);
            } else {
                return unsafe { from_utf8_unchecked(&text[pos[2].unwrap_unchecked() + 1..i]) };
            }
        }
    }
    unreachable!();
}

// #[inline]
// fn db_to_amplitude(db: f32) -> f32 {
//     10.0_f32.powf(db / 20.0)
// }

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Song {
    pub name: String,
    pub album: String,
    pub artist: String,
    pub path: String,
    pub number: u8,
    pub disc: u8,
    pub gain: f32,
    pub id: usize,
}

impl Song {
    fn from(bytes: &[u8], id: usize) -> Self {
        let text = &bytes[..TEXT_LEN];
        Self {
            name: name(text).to_string(),
            album: album(text).to_string(),
            artist: artist(text).to_string(),
            path: path(text).to_string(),
            number: bytes[SONG_LEN - 2],
            disc: bytes[SONG_LEN - 1],
            gain: 0.0,
            id,
        }
    }
}

pub struct RawSong {
    //Name, album, artist, path are all crammed into this space.
    text: [u8; TEXT_LEN],
    number: u8,
    disc: u8,
}

impl RawSong {
    pub fn new(name: &str, album: &str, artist: &str, path: &str, number: u8, disc: u8) -> Self {
        let len = name.len() + album.len() + artist.len() + path.len();
        if len > TEXT_LEN {
            panic!("Text is '{}' bytes to many!", len - TEXT_LEN);
        } else {
            let name = [name.as_bytes(), &[b'\0']].concat();
            let album = [album.as_bytes(), &[b'\0']].concat();
            let artist = [artist.as_bytes(), &[b'\0']].concat();
            let path = [path.as_bytes(), &[b'\0']].concat();

            let mut text = [0u8; TEXT_LEN];
            let name_pos = name.len();
            let album_pos = name_pos + album.len();
            let artist_pos = album_pos + artist.len();
            let path_pos = artist_pos + path.len();

            text[..name_pos].copy_from_slice(&name);
            text[name_pos..album_pos].copy_from_slice(&album);
            text[album_pos..artist_pos].copy_from_slice(&artist);
            text[artist_pos..path_pos].copy_from_slice(&path);

            Self { text, number, disc }
        }
    }
    pub fn into_bytes(self) -> [u8; SONG_LEN] {
        let mut song = [0u8; SONG_LEN];
        song[0..TEXT_LEN].copy_from_slice(&self.text);
        song[SONG_LEN - 2] = self.number;
        song[SONG_LEN - 1] = self.disc;
        song
    }
}

impl Debug for RawSong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = name(&self.text);
        let album = album(&self.text);
        let artist = artist(&self.text);
        let path = path(&self.text);
        f.debug_struct("Song")
            .field("name", &name)
            .field("album", &album)
            .field("artist", &artist)
            .field("path", &path)
            .field("number", &self.number)
            .field("disc", &self.disc)
            .finish()
    }
}

impl From<&'_ [u8]> for RawSong {
    fn from(bytes: &[u8]) -> Self {
        Self {
            text: bytes[..TEXT_LEN].try_into().unwrap(),
            number: bytes[SONG_LEN - 2],
            disc: bytes[SONG_LEN - 1],
        }
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

        let mut name = String::from("Unknown Title");
        let mut album = String::from("Unknown Album");
        let mut artist = String::from("Unknown Artist");
        let mut number = 1;
        let mut disc = 1;

        let mut update_metadata = |metadata: &MetadataRevision| {
            for tag in metadata.tags() {
                if let Some(std_key) = tag.std_key {
                    match std_key {
                        StandardTagKey::AlbumArtist => artist = tag.value.to_string(),
                        StandardTagKey::Artist if artist == "Unknown Artist" => {
                            artist = tag.value.to_string()
                        }
                        StandardTagKey::Album => album = tag.value.to_string(),
                        StandardTagKey::TrackTitle => name = tag.value.to_string(),
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

        RawSong::new(
            &name,
            &album,
            &artist,
            &path.to_string_lossy(),
            number,
            disc,
        )
    }
}

static mut MMAP: Option<Mmap> = None;

fn mmap() -> &'static Mmap {
    unsafe { MMAP.as_ref().unwrap_unchecked() }
}

pub fn init() {
    let gonk = if cfg!(windows) {
        PathBuf::from(&env::var("APPDATA").unwrap())
    } else {
        PathBuf::from(&env::var("HOME").unwrap()).join(".config")
    }
    .join("gonk");

    let db_path = gonk.join("gonk.db");
    let db_exists = db_path.exists();

    if !gonk.exists() {
        fs::create_dir_all(&gonk).unwrap();
    }

    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&db_path)
        .unwrap();

    if !db_exists {
        let mut writer = BufWriter::new(&file);

        let paths: Vec<DirEntry> = walkdir::WalkDir::new("D:\\OneDrive\\Music")
            .into_iter()
            .flatten()
            .filter(|path| match path.path().extension() {
                Some(ex) => {
                    matches!(ex.to_str(), Some("flac" | "mp3" | "ogg" | "wav" | "m4a"))
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

        // let song = RawSong::new(
        //     "joe's song",
        //     "joe's album",
        //     "joe",
        //     "D:\\OneDrive\\Joe\\joe's song.flac",
        //     2,
        //     1,
        // );
        // let bytes = song.into_bytes();
        // for _ in 0..100_000 {
        //     writer.write_all(&bytes).unwrap();
        // }

        writer.flush().unwrap();
    }

    unsafe { MMAP = Some(Mmap::map(&file).unwrap()) };
}

pub fn get(index: usize) -> Option<Song> {
    let start = SONG_LEN * index;
    let bytes = mmap().get(start..start + SONG_LEN)?;
    Some(Song::from(bytes, index))
}

pub fn ids(ids: &[usize]) -> Vec<Song> {
    let mmap = mmap();
    let mut songs = Vec::new();
    for id in ids {
        let start = SONG_LEN * id;
        let bytes = &mmap[start..start + SONG_LEN];
        songs.push(Song::from(bytes, *id));
    }
    songs
}

pub fn par_ids(ids: &[usize]) -> Vec<Song> {
    let mmap = mmap();
    ids.par_iter()
        .map(|id| {
            let start = SONG_LEN * id;
            let bytes = &mmap[start..start + SONG_LEN];
            Song::from(bytes, *id)
        })
        .collect()
}

pub fn songs_from_album(al: &str, ar: &str) -> Vec<Song> {
    let mmap = mmap();
    let mut songs = Vec::new();
    let mut i = 0;
    while let Some(text) = mmap.get(i..i + TEXT_LEN) {
        let album = album(text);
        let artist = artist(text);
        if album == al && artist == ar {
            songs.push(Song::from(&mmap[i..i + SONG_LEN], i / SONG_LEN));
        }
        i += SONG_LEN;
    }
    songs
}

pub fn par_songs_from_album(al: &str, ar: &str) -> Vec<Song> {
    let mmap = mmap();
    (0..len())
        .into_par_iter()
        .filter_map(|i| {
            let pos = i * SONG_LEN;
            let text = &mmap[pos..pos + TEXT_LEN];
            let album = album(text);
            let artist = artist(text);
            if album == al && artist == ar {
                Some(Song::from(&mmap[pos..pos + SONG_LEN], i / SONG_LEN))
            } else {
                None
            }
        })
        .collect()
}

pub fn names_from_album(gonk_database: &str) -> Vec<String> {
    let mmap = mmap();
    let mut songs = Vec::new();
    let mut i = 0;
    while let Some(text) = mmap.get(i..i + TEXT_LEN) {
        let album = album(text);
        if album == gonk_database {
            let number = mmap[i + SONG_LEN - 2];
            songs.push(format!("{}. {}", number, name(text)));
        }
        i += SONG_LEN;
    }
    songs
}

pub fn par_names_from_album(gonk_database: &str) -> Vec<String> {
    let mmap = mmap();
    (0..len())
        .into_par_iter()
        .filter_map(|i| {
            let pos = i * SONG_LEN;
            let text = &mmap[pos..pos + TEXT_LEN];
            let album = album(text);
            if album == gonk_database {
                let number = &mmap[pos + SONG_LEN - 2];
                Some(format!("{}. {}", number, name(text)))
            } else {
                None
            }
        })
        .collect()
}

pub fn albums_by_artist(gonk_database: &str) -> Vec<String> {
    let mmap = mmap();
    let mut albums = Vec::new();
    let mut i = 0;
    while let Some(text) = mmap.get(i..i + TEXT_LEN) {
        let artist = artist(text);
        if artist == gonk_database {
            albums.push(album(text).to_string());
        }
        i += SONG_LEN;
    }
    albums.sort_unstable();
    albums.dedup();
    albums
}

pub fn par_albums_by_artist(gonk_database: &str) -> Vec<String> {
    let mmap = mmap();
    let mut albums: Vec<String> = (0..len())
        .into_par_iter()
        .filter_map(|i| {
            let pos = i * SONG_LEN;
            let text = &mmap[pos..pos + TEXT_LEN];
            let artist = artist(text);
            if artist == gonk_database {
                Some(album(text).to_string())
            } else {
                None
            }
        })
        .collect();
    albums.sort_unstable();
    albums.dedup();
    albums
}

pub fn songs_by_artist(gonk_database: &str) -> Vec<Song> {
    let mmap = mmap();
    let mut songs = Vec::new();
    let mut i = 0;
    while let Some(text) = mmap.get(i..i + TEXT_LEN) {
        let artist = artist(text);
        if artist == gonk_database {
            let song_bytes = &mmap[i..i + SONG_LEN];
            songs.push(Song::from(song_bytes, i / SONG_LEN));
        }
        i += SONG_LEN;
    }
    songs
}

pub fn par_songs_by_artist(gonk_database: &str) -> Vec<Song> {
    let mmap = mmap();
    (0..len())
        .into_par_iter()
        .filter_map(|i| {
            let pos = i * SONG_LEN;
            let text = &mmap[pos..pos + TEXT_LEN];
            let artist = artist(text);
            if artist == gonk_database {
                let song_bytes = &mmap[pos..pos + SONG_LEN];
                Some(Song::from(song_bytes, i / SONG_LEN))
            } else {
                None
            }
        })
        .collect()
}

pub fn songs() -> Vec<Song> {
    let mmap = mmap();
    let mut songs = Vec::new();
    let mut i = 0;
    while let Some(bytes) = mmap.get(i..i + SONG_LEN) {
        songs.push(Song::from(bytes, i / SONG_LEN));
        i += SONG_LEN;
    }
    songs
}

///(Album, Artist)
pub fn albums() -> Vec<(String, String)> {
    let mmap = mmap();
    let mut albums = Vec::new();
    let mut i = 0;
    while let Some(text) = mmap.get(i..i + TEXT_LEN) {
        albums.push((album(text).to_string(), artist(text).to_string()));
        i += SONG_LEN;
    }
    albums.sort_unstable();
    albums.dedup();
    albums
}

pub fn artists() -> Vec<String> {
    let mmap = mmap();
    let mut artists = Vec::new();
    let mut i = 0;
    while let Some(text) = mmap.get(i..i + TEXT_LEN) {
        artists.push(artist(text).to_string());
        i += SONG_LEN;
    }
    artists.sort_unstable();
    artists.dedup();
    artists
}

pub fn par_artists() -> Vec<String> {
    let mmap = mmap();
    let mut artists: Vec<String> = (0..len())
        .into_par_iter()
        .map(|i| {
            let pos = i * SONG_LEN;
            let text = &mmap[pos..pos + TEXT_LEN];
            artist(text).to_string()
        })
        .collect();
    artists.par_sort_unstable();
    artists.dedup();
    artists
}

pub fn len() -> usize {
    mmap().len() / SONG_LEN
}

pub fn bench<F>(func: F)
where
    F: FnOnce(),
{
    let now = Instant::now();
    func();
    println!("{:?}", now.elapsed());
}

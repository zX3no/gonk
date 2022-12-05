use crate::Song;
use crate::{flac_decoder::read_metadata, settings::Settings, *};
use crate::{log, profile};
use memmap2::Mmap;
use once_cell::unsync::Lazy;
use rayon::{
    prelude::{
        IntoParallelIterator, IntoParallelRefIterator, ParallelDrainRange, ParallelIterator,
    },
    slice::ParallelSliceMut,
};
use std::ffi::OsString;
use std::str::from_utf8;
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    path::Path,
    sync::RwLock,
    thread::{self, JoinHandle},
};
use std::{fmt::Debug, mem::size_of};
use symphonia::{
    core::{
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::{Limit, MetadataOptions, MetadataRevision, StandardTagKey},
        probe::Hint,
    },
    default::get_probe,
};
use walkdir::{DirEntry, WalkDir};

const MIN_ACCURACY: f64 = 0.70;

//TODO: Rework
//TODO: Could this be stored in a more drive friendly size like 512?
//522 + 1 + 1 + 4 = 528
pub const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() + size_of::<u8>() + size_of::<f32>();
pub const TEXT_LEN: usize = 522;
pub const NUMBER_POS: usize = SONG_LEN - 1 - size_of::<f32>() - size_of::<u8>();
pub const DISC_POS: usize = SONG_LEN - 1 - size_of::<f32>();
pub const GAIN_POS: Range<usize> = SONG_LEN - size_of::<f32>()..SONG_LEN;

pub static mut DB: Lazy<Database> = Lazy::new(|| unsafe { Database::new() });

#[derive(Clone, Debug)]
pub enum Item {
    ///(Artist, Album, Name, Disc Number, Track Number)
    Song((&'static String, &'static String, &'static String, u8, u8)),
    ///(Artist, Album)
    Album((&'static String, &'static String)),
    ///(Artist)
    Artist(&'static String),
}

pub enum ScanResult {
    Completed,
    CompletedWithErrors(Vec<String>),
    Incomplete(&'static str),
}

pub fn bytes_to_song(bytes: &[u8]) -> Song {
    debug_assert!(bytes.len() == SONG_LEN);

    let text = bytes.get(..TEXT_LEN).unwrap();
    debug_assert!(text.len() == TEXT_LEN);

    let artist_len = u16::from_le_bytes([text[0], text[1]]) as usize;
    let album_len = u16::from_le_bytes([text[2 + artist_len], text[2 + artist_len + 1]]) as usize;
    let title_len = u16::from_le_bytes([
        text[2 + artist_len + 2 + album_len],
        text[2 + artist_len + 2 + album_len + 1],
    ]) as usize;
    let path_len = u16::from_le_bytes([
        text[2 + artist_len + 2 + album_len + 2 + title_len],
        text[2 + artist_len + 2 + album_len + 2 + title_len + 1],
    ]) as usize;

    let slice = text.get(2..artist_len + 2).unwrap();
    let artist = from_utf8(slice).unwrap();

    let slice = text
        .get(2 + artist_len + 2..2 + artist_len + 2 + album_len)
        .unwrap();
    let album = from_utf8(slice).unwrap();

    let slice = text
        .get(2 + artist_len + 2..2 + artist_len + 2 + album_len)
        .unwrap();
    let title = from_utf8(slice).unwrap();

    let slice = text
        .get(
            2 + artist_len + 2 + album_len + 2 + title_len + 2
                ..2 + artist_len + 2 + album_len + 2 + title_len + 2 + path_len,
        )
        .unwrap();
    let path = from_utf8(slice).unwrap();

    let track_number = bytes[NUMBER_POS];
    let disc_number = bytes[DISC_POS];

    let gain = f32::from_le_bytes([
        bytes[SONG_LEN - 4],
        bytes[SONG_LEN - 3],
        bytes[SONG_LEN - 2],
        bytes[SONG_LEN - 1],
    ]);

    Song {
        title: title.to_string(),
        album: album.to_string(),
        artist: artist.to_string(),
        disc_number,
        track_number,
        path: path.to_string(),
        gain,
    }
}

pub fn song_to_bytes(
    artist: &str,
    album: &str,
    title: &str,
    path: &str,
    number: u8,
    disc: u8,
    gain: f32,
) -> [u8; SONG_LEN] {
    let mut song = [0; SONG_LEN];

    if path.len() > TEXT_LEN {
        //If there is invalid utf8 in the panic message, rust will panic.
        panic!("PATH IS TOO LONG! {:?}", OsString::from(path));
    }

    let mut artist = artist.to_string();
    let mut album = album.to_string();
    let mut title = title.to_string();

    //Forcefully fit the artist, album, title and path into 522 bytes.
    //There are 4 u16s included in the text so those are subtracted too.
    let mut i = 0;
    while artist.len() + album.len() + title.len() + path.len() > TEXT_LEN - (4 * size_of::<u16>())
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

    let artist_len = (artist.len() as u16).to_le_bytes();
    song[0..2].copy_from_slice(&artist_len);
    song[2..2 + artist.len()].copy_from_slice(artist.as_bytes());

    let album_len = (album.len() as u16).to_le_bytes();
    song[2 + artist.len()..2 + artist.len() + 2].copy_from_slice(&album_len);
    song[2 + artist.len() + 2..2 + artist.len() + 2 + album.len()]
        .copy_from_slice(album.as_bytes());

    let title_len = (title.len() as u16).to_le_bytes();
    song[2 + artist.len() + 2 + album.len()..2 + artist.len() + 2 + album.len() + 2]
        .copy_from_slice(&title_len);
    song[2 + artist.len() + 2 + album.len() + 2
        ..2 + artist.len() + 2 + album.len() + 2 + title.len()]
        .copy_from_slice(title.as_bytes());

    let path_len = (path.len() as u16).to_le_bytes();
    song[2 + artist.len() + 2 + album.len() + 2 + title.len()
        ..2 + artist.len() + 2 + album.len() + 2 + title.len() + 2]
        .copy_from_slice(&path_len);
    song[2 + artist.len() + 2 + album.len() + 2 + title.len() + 2
        ..2 + artist.len() + 2 + album.len() + 2 + title.len() + 2 + path.len()]
        .copy_from_slice(path.as_bytes());

    song[NUMBER_POS] = number;
    song[DISC_POS] = disc;
    song[GAIN_POS].copy_from_slice(&gain.to_le_bytes());

    song
}

pub fn path_to_bytes(path: &'_ Path) -> Result<[u8; SONG_LEN], String> {
    let ex = path.extension().unwrap();
    if ex == "flac" {
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

                Ok(song_to_bytes(
                    artist,
                    album,
                    title,
                    path.to_str().unwrap(),
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

        Ok(song_to_bytes(
            &artist,
            &album,
            &title,
            path.to_str().unwrap(),
            number,
            disc,
            gain,
        ))
    }
}

pub struct Database {
    pub data: BTreeMap<String, Vec<Album>>,
    pub mmap: Option<Mmap>,
    pub settings: Settings,
}

impl Database {
    //This should replace gonk_core::init();
    pub unsafe fn new() -> Self {
        //We only need write access to create the file.
        let db = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(database_path())
            .unwrap();

        let mmap = Mmap::map(&db).unwrap();
        let mut data = BTreeMap::new();

        Database::build(&mmap, &mut data);

        Self {
            data,
            mmap: Some(mmap),
            settings: Settings::new(),
        }
    }

    pub fn len() -> usize {
        unsafe { DB.mmap.as_ref().unwrap().len() / SONG_LEN }
    }

    ///Reset the database.
    pub unsafe fn reset() -> io::Result<()> {
        DB.mmap = None;
        DB.settings.file.set_len(0)?;
        fs::remove_file(database_path())?;
        Ok(())
    }

    ///Refresh the in memory database with songs from the physical one.
    pub unsafe fn build(mmap: &Mmap, data: &mut BTreeMap<String, Vec<Album>>) {
        //TODO: Maybe do this on another thread.
        //Waiting could be quite costly for large libraries.

        //Load all songs into memory.
        let songs: Vec<Song> = (0..mmap.len() / SONG_LEN)
            .into_par_iter()
            .map(|i| {
                let pos = i * SONG_LEN;
                let bytes = &mmap[pos..pos + SONG_LEN];
                bytes_to_song(bytes)
            })
            .collect();

        // dbg!(&songs);

        let mut albums: BTreeMap<(String, String), Vec<Song>> = BTreeMap::new();

        //Add songs to albums.
        for song in songs {
            match albums.entry((song.artist.clone(), song.album.clone())) {
                Entry::Occupied(mut entry) => entry.get_mut().push(song),
                Entry::Vacant(entry) => {
                    entry.insert(vec![song]);
                }
            }
        }

        //Sort songs.
        albums.iter_mut().for_each(|(_, album)| {
            album.sort_unstable_by(|a, b| {
                if a.disc_number == b.disc_number {
                    a.track_number.cmp(&b.track_number)
                } else {
                    a.disc_number.cmp(&b.disc_number)
                }
            });
        });

        //Add albums to artists.
        for ((artist, album), v) in albums {
            let v = Album {
                title: album,
                songs: v,
            };
            match data.entry(artist) {
                Entry::Occupied(mut entry) => entry.get_mut().push(v),
                Entry::Vacant(entry) => {
                    entry.insert(vec![v]);
                }
            }
        }

        //Sort albums.
        data.iter_mut().for_each(|(_, albums)| {
            albums.sort_unstable_by_key(|album| album.title.to_ascii_lowercase());
        });
    }

    /// Collect songs and add them to the database.
    pub fn scan(path: String) -> JoinHandle<ScanResult> {
        if let Some(mmap) = unsafe { DB.mmap.take() } {
            drop(mmap);
        }

        thread::spawn(|| {
            let mut db_path = database_path();
            db_path.pop();
            db_path.push("temp.db");

            match File::create(&db_path) {
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

                    let songs: Vec<_> = paths
                        .into_par_iter()
                        .map(|dir| {
                            //
                            let bytes = path_to_bytes(dir.path());
                            dbg!(bytes_to_song(bytes.as_ref().unwrap()));

                            bytes
                        })
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

                    let songs: Vec<_> = songs.into_iter().flatten().collect();

                    for song in songs {
                        writer.write_all(&song).unwrap();
                    }

                    writer.flush().unwrap();

                    //Remove old database and replace it with new.
                    fs::rename(db_path, database_path()).unwrap();

                    unsafe {
                        let db = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(database_path())
                            .unwrap();
                        let mmap = Mmap::map(&db).unwrap();
                        Database::build(&mmap, &mut DB.data);
                        DB.mmap = Some(mmap);
                    };

                    if errors.is_empty() {
                        ScanResult::Completed
                    } else {
                        ScanResult::CompletedWithErrors(errors)
                    }
                }
                Err(_) => {
                    //Re-open the database as read only.
                    let db = OpenOptions::new().read(true).open(database_path()).unwrap();
                    unsafe { DB.mmap = Some(Mmap::map(&db).unwrap()) };
                    ScanResult::Incomplete("Failed to scan folder, database is already open.")
                }
            }
        })
    }

    //Browser Queries:

    ///Get all aritist names.
    pub fn artists() -> Vec<&'static String> {
        let db = unsafe { &DB.data };
        let mut v = Vec::from_iter(db.keys());
        v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
        v
    }

    ///Get all albums by an artist.
    pub fn albums_by_artist(artist: &str) -> Vec<&'static Album> {
        let db = unsafe { &DB.data };
        if let Some(albums) = db.get(artist) {
            albums.iter().collect()
        } else {
            Vec::new()
        }
    }

    ///Get album by artist and album name.
    pub fn album(artist: &str, album: &str) -> Option<&'static Album> {
        let db = unsafe { &DB.data };
        if let Some(albums) = db.get(artist) {
            for al in albums {
                if album == al.title {
                    return Some(al);
                }
            }
        }
        None
    }

    ///Get an individual song in the database.
    pub fn song(artist: &str, album: &str, disc: u8, number: u8) -> Option<&'static Song> {
        let db = unsafe { &DB.data };

        if let Some(albums) = db.get(artist) {
            for al in albums {
                if al.title == album {
                    for song in &al.songs {
                        if song.disc_number == disc && song.track_number == number {
                            return Some(song);
                        }
                    }
                }
            }
        }

        None
    }

    ///Get albums by aritist.
    pub fn artist(artist: &str) -> Option<&'static Vec<Album>> {
        let db = unsafe { &DB.data };
        db.get(artist)
    }

    //Search Queries:

    ///Search the database and return the 25 most accurate matches.
    pub fn search(query: &str) -> Vec<Item> {
        let db = unsafe { &DB.data };
        let query = query.to_lowercase();
        let results = RwLock::new(Vec::new());

        db.par_iter().for_each(|(artist, albums)| {
            for album in albums {
                for song in &album.songs {
                    let song = jaro(
                        &query,
                        Item::Song((
                            artist,
                            &album.title,
                            &song.title,
                            song.disc_number,
                            song.track_number,
                        )),
                    );
                    results.write().unwrap().push(song);
                }
                let album = jaro(&query, Item::Album((artist, &album.title)));
                results.write().unwrap().push(album);
            }
            let artist = jaro(&query, Item::Artist(artist));
            results.write().unwrap().push(artist);
        });

        let results = RwLock::into_inner(results).unwrap();

        let mut results: Vec<_> = if query.is_empty() {
            results
                .into_iter()
                .take(25)
                .filter_map(|x| match x {
                    Ok(_) => None,
                    Err(x) => Some(x),
                })
                .collect()
        } else {
            results.into_iter().flatten().collect()
        };

        //Sort results by score.
        results.par_sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        if results.len() > 25 {
            //Remove the less accurate results.
            results.par_drain(25..);
        }

        results.sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
            if score_1 == score_2 {
                match item_1 {
                    Item::Artist(_) => match item_2 {
                        Item::Song(_) | Item::Album(_) => Ordering::Less,
                        Item::Artist(_) => Ordering::Equal,
                    },
                    Item::Album(_) => match item_2 {
                        Item::Song(_) => Ordering::Less,
                        Item::Album(_) => Ordering::Equal,
                        Item::Artist(_) => Ordering::Greater,
                    },
                    Item::Song((_, _, _, disc_a, number_a)) => match item_2 {
                        Item::Song((_, _, _, disc_b, number_b)) => match disc_a.cmp(disc_b) {
                            Ordering::Less => Ordering::Less,
                            Ordering::Equal => number_a.cmp(number_b),
                            Ordering::Greater => Ordering::Greater,
                        },
                        Item::Album(_) | Item::Artist(_) => Ordering::Greater,
                    },
                }
            } else if score_2 > score_1 {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        });

        results.into_iter().map(|(item, _)| item).collect()
    }

    //Settings:
    pub fn output_device() -> &'static str {
        unsafe { &DB.settings.output_device }
    }

    pub fn music_folder() -> &'static str {
        unsafe { &DB.settings.music_folder }
    }

    pub fn volume() -> u8 {
        unsafe { DB.settings.volume }
    }

    pub fn set_volume(volume: u8) {
        unsafe {
            DB.settings.volume = volume;
            DB.settings.save().unwrap();
        }
    }

    pub fn queue() -> (&'static Vec<Song>, Option<usize>, f32) {
        let settings = unsafe { &DB.settings };
        let index = if settings.queue.is_empty() {
            None
        } else {
            Some(settings.index as usize)
        };

        (&settings.queue, index, settings.elapsed)
    }

    pub fn save_queue(queue: &[Song], index: u16, elapsed: f32) {
        unsafe {
            DB.settings.queue = queue.to_vec();
            DB.settings.index = index;
            DB.settings.elapsed = elapsed;
            DB.settings.save().unwrap();
        };
    }

    pub fn update_queue_state(index: u16, elapsed: f32) {
        unsafe {
            DB.settings.elapsed = elapsed;
            DB.settings.index = index;
            DB.settings.save().unwrap();
        }
    }

    pub fn update_output_device(device: &str) {
        unsafe {
            DB.settings.output_device = device.to_string();
            DB.settings.save().unwrap();
        }
    }

    pub fn update_music_folder(folder: &str) {
        unsafe {
            DB.settings.music_folder = folder.replace('\\', "/");
            DB.settings.save().unwrap();
        }
    }

    pub fn raw() -> &'static BTreeMap<String, Vec<Album>> {
        unsafe { &DB.data }
    }
}

pub fn jaro(query: &str, input: Item) -> Result<(Item, f64), (Item, f64)> {
    let str = match input {
        Item::Artist(artist) => artist,
        Item::Album((_, album)) => album,
        Item::Song((_, _, song, _, _)) => song,
    };
    let acc = strsim::jaro_winkler(query, &str.to_lowercase());
    if acc > MIN_ACCURACY {
        Ok((input, acc))
    } else {
        Err((input, acc))
    }
}

mod strsim {
    use std::cmp::{max, min};

    pub fn jaro_winkler(a: &str, b: &str) -> f64 {
        let jaro_distance = generic_jaro(a, b);

        // Don't limit the length of the common prefix
        let prefix_length = a
            .chars()
            .zip(b.chars())
            .take_while(|(a_elem, b_elem)| a_elem == b_elem)
            .count();

        let jaro_winkler_distance =
            jaro_distance + (0.15 * prefix_length as f64 * (1.0 - jaro_distance));

        jaro_winkler_distance.clamp(0.0, 1.0)
    }

    pub fn generic_jaro(a: &str, b: &str) -> f64 {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        // The check for lengths of one here is to prevent integer overflow when
        // calculating the search range.
        if a_len == 0 && b_len == 0 {
            return 1.0;
        } else if a_len == 0 || b_len == 0 {
            return 0.0;
        } else if a_len == 1 && b_len == 1 {
            return if a.chars().eq(b.chars()) { 1.0 } else { 0.0 };
        }

        let search_range = (max(a_len, b_len) / 2) - 1;

        let mut b_consumed = vec![false; b_len];
        let mut matches = 0.0;

        let mut transpositions = 0.0;
        let mut b_match_index = 0;

        for (i, a_elem) in a.chars().enumerate() {
            let min_bound =
            // prevent integer wrapping
            if i > search_range {
                max(0, i - search_range)
            } else {
                0
            };

            let max_bound = min(b_len - 1, i + search_range);

            if min_bound > max_bound {
                continue;
            }

            for (j, b_elem) in b.chars().enumerate() {
                if min_bound <= j && j <= max_bound && a_elem == b_elem && !b_consumed[j] {
                    b_consumed[j] = true;
                    matches += 1.0;

                    if j < b_match_index {
                        transpositions += 1.0;
                    }
                    b_match_index = j;

                    break;
                }
            }
        }

        if matches == 0.0 {
            0.0
        } else {
            (1.0 / 3.0)
                * ((matches / a_len as f64)
                    + (matches / b_len as f64)
                    + ((matches - transpositions) / matches))
        }
    }
}

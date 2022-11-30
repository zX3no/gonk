use crate::{settings::Settings, *};
use memmap2::Mmap;
use once_cell::unsync::Lazy;
use rayon::{
    prelude::{
        IntoParallelIterator, IntoParallelRefIterator, ParallelDrainRange, ParallelIterator,
    },
    slice::ParallelSliceMut,
};
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
    sync::RwLock,
    thread::{self, JoinHandle},
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

#[derive(Debug)]
pub struct Artist {
    pub albums: Vec<Album>,
}

#[derive(Debug)]
pub struct Album {
    pub title: String,
    pub songs: Vec<Song>,
}

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

impl Song {
    pub fn from(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() == SONG_LEN);
        let text = unsafe { bytes.get_unchecked(..TEXT_LEN) };
        let artist = artist(text);
        let album = album(text);
        let title = title(text);
        let path = path(text);

        let track_number = bytes[NUMBER_POS];
        let disc_number = bytes[DISC_POS];

        let gain = f32::from_le_bytes([
            bytes[SONG_LEN - 4],
            bytes[SONG_LEN - 3],
            bytes[SONG_LEN - 2],
            bytes[SONG_LEN - 1],
        ]);

        Self {
            artist: artist.to_string(),
            album: album.to_string(),
            title: title.to_string(),
            path: path.to_string(),
            track_number,
            disc_number,
            gain,
        }
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
        let mut settings = match fs::read(settings_path()) {
            Ok(bytes) if !bytes.is_empty() => match Settings::from(bytes) {
                Some(settings) => settings,
                //Save the default settings when they are empty.
                None => Settings::default().write().unwrap(),
            },
            //Save the default settings if nothing is found.
            _ => Settings::default().write().unwrap(),
        };

        //We only need write access to create the file.
        let db = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(database_path())
            .unwrap();

        let mmap = Mmap::map(&db).unwrap();
        let mut data = BTreeMap::new();

        //Reset the database if the first song is invalid.
        let mmap = if Database::validate(&mmap).is_err() {
            log!("Database is corrupted. Resetting!");
            settings = Settings::default().write().unwrap();
            fs::remove_file(settings_path()).unwrap();
            fs::remove_file(database_path()).unwrap();
            None
        } else {
            Database::build(&mmap, &mut data);
            Some(mmap)
        };

        Self {
            data,
            mmap,
            settings,
        }
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
            &text[2 + artist_len + 2 + album_len + 2
                ..artist_len + 2 + album_len + 2 + title_len + 2],
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

    pub fn len() -> usize {
        unsafe { DB.mmap.as_ref().unwrap().len() / SONG_LEN }
    }

    ///Reset the database.
    pub unsafe fn reset() -> io::Result<()> {
        DB.mmap = None;
        DB.settings = Settings::default();
        fs::remove_file(settings_path())?;
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
                Song::from(bytes)
            })
            .collect();

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
                        .map(|path| RawSong::from_path(path.path()))
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

                    let songs: Vec<RawSong> = songs.into_iter().flatten().collect();

                    for song in songs {
                        writer.write_all(&song.as_bytes()).unwrap();
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

        //TODO: Cleanup
        if query.is_empty() {
            for (artist, albums) in db {
                results.write().unwrap().push((Item::Artist(artist), 1.0));
                for album in albums {
                    results
                        .write()
                        .unwrap()
                        .push((Item::Album((artist, &album.title)), 1.0));
                    for song in &album.songs {
                        results.write().unwrap().push((
                            Item::Song((
                                artist,
                                &album.title,
                                &song.title,
                                song.disc_number,
                                song.track_number,
                            )),
                            1.0,
                        ));
                    }
                }
            }
        } else {
            db.par_iter().for_each(|(artist, albums)| {
                if let Some(result) = calc(&query, Item::Artist(artist)) {
                    results.write().unwrap().push(result);
                }

                for album in albums {
                    if let Some(result) = calc(&query, Item::Album((artist, &album.title))) {
                        results.write().unwrap().push(result);
                    }

                    results.write().unwrap().extend(
                        album
                            .songs
                            .iter()
                            .filter_map(|song| {
                                calc(
                                    &query,
                                    Item::Song((
                                        artist,
                                        &album.title,
                                        &song.title,
                                        song.disc_number,
                                        song.track_number,
                                    )),
                                )
                            })
                            .collect::<Vec<(Item, f64)>>(),
                    );
                }
            });
        }

        let mut results = RwLock::into_inner(results).unwrap();

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

    pub fn save_volume(new_volume: u8) {
        unsafe {
            DB.settings.volume = new_volume;
            DB.settings.save().unwrap();
        }
    }

    pub fn save_queue(queue: &[Song], index: u16, elapsed: f32) {
        unsafe {
            DB.settings.queue = queue.iter().map(RawSong::from).collect();
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

    pub fn get_saved_queue() -> (Vec<Song>, Option<usize>, f32) {
        let settings = unsafe { &DB.settings };
        let index = if settings.queue.is_empty() {
            None
        } else {
            Some(settings.index as usize)
        };

        (
            settings
                .queue
                .iter()
                .map(|song| Song::from(&song.as_bytes()))
                .collect(),
            index,
            settings.elapsed,
        )
    }

    pub fn output_device() -> &'static str {
        unsafe { &DB.settings.output_device }
    }

    pub fn music_folder() -> &'static str {
        unsafe { &DB.settings.music_folder }
    }

    pub fn volume() -> u8 {
        unsafe { DB.settings.volume }
    }

    pub unsafe fn raw() -> &'static BTreeMap<String, Vec<Album>> {
        &DB.data
    }
}

pub fn calc(query: &str, input: Item) -> Option<(Item, f64)> {
    let str = match input {
        Item::Artist(artist) => artist,
        Item::Album((_, album)) => album,
        Item::Song((_, _, song, _, _)) => song,
    };
    let acc = strsim::jaro_winkler(query, &str.to_lowercase());
    if acc > MIN_ACCURACY {
        Some((input, acc))
    } else {
        None
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
            .take_while(|&(ref a_elem, ref b_elem)| a_elem == b_elem)
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

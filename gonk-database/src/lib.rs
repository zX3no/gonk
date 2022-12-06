pub use flac_decoder::*;
use rayon::{
    prelude::{
        IntoParallelIterator, IntoParallelRefIterator, ParallelDrainRange, ParallelIterator,
    },
    slice::ParallelSliceMut,
};
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
    env,
    error::Error,
    ffi::OsString,
    fs::{self, File},
    io::{BufWriter, Write},
    mem::size_of,
    ops::Range,
    path::{Path, PathBuf},
    str::from_utf8,
    sync::{Once, RwLock},
    thread::{self, JoinHandle},
};
use walkdir::{DirEntry, WalkDir};

pub mod flac_decoder;
pub mod lazy;
pub mod strsim;

pub use lazy::*;

pub const SONG_LEN: usize = TEXT_LEN + size_of::<u8>() + size_of::<u8>() + size_of::<f32>();
pub const TEXT_LEN: usize = 522;
pub const NUMBER_POS: usize = SONG_LEN - 1 - size_of::<f32>() - size_of::<u8>();
pub const DISC_POS: usize = SONG_LEN - 1 - size_of::<f32>();
pub const GAIN_POS: Range<usize> = SONG_LEN - size_of::<f32>()..SONG_LEN;

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

#[derive(Clone, Debug)]
pub enum Item<'a> {
    ///(Artist, Album, Name, Disc Number, Track Number)
    Song((&'a String, &'a String, &'a String, u8, u8)),
    ///(Artist, Album)
    Album((&'a String, &'a String)),
    ///(Artist)
    Artist(&'a String),
}

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
    let mut path = gonk_path();
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

pub fn bytes_to_song(bytes: &[u8]) -> Result<Song, Box<dyn Error + Send + Sync>> {
    if bytes.len() != SONG_LEN {
        return Err("Slice size does not match song length")?;
    }

    //Find the positions of the data.

    let Some(text) = bytes.get(..TEXT_LEN) else {
        return Err("Could not get text segment")?;
    };

    let artist_len = u16::from_le_bytes([text[0], text[1]]) as usize;

    let Some(slice) = text.get(2 + artist_len..2 + artist_len + 2) else{
        return Err("Artist length is corrupted")?;
    };

    let album_len = u16::from_le_bytes(slice.try_into()?) as usize;

    let Some(slice) = text.get(2 + artist_len + 2 + album_len..2 + artist_len + 2 + album_len + 2) else{
        return Err("Album length is corrupted")?;
    };

    let title_len = u16::from_le_bytes(slice.try_into()?) as usize;

    let Some(slice) = text.get(2 + artist_len + 2 + album_len + 2 + title_len..2 + artist_len + 2 + album_len + 2 + title_len + 2) else{
        return Err("Title length is corrupted")?;
    };

    let path_len = u16::from_le_bytes(slice.try_into()?) as usize;

    //Collect the data.

    let slice = text.get(2..artist_len + 2).ok_or("Invalid artist length")?;
    let artist = from_utf8(slice)?;

    let Some(slice) = text.get(2 + artist_len + 2..2 + artist_len + 2 + album_len) else{
        return Err("Invalid album length")?;
    };
    let album = from_utf8(slice)?;

    let Some(slice) = text.get(2 + artist_len + 2 + album_len + 2..2 + artist_len + 2 + album_len + 2 + title_len) else{
        return Err("Invalid title length")?;
    };
    let title = from_utf8(slice)?;

    let Some(slice) = text.get(2 + artist_len + 2 + album_len + 2 + title_len + 2
                ..2 + artist_len + 2 + album_len + 2 + title_len + 2 + path_len) else {
        return Err("Invalid path length")?;
    };
    let path = from_utf8(slice)?;

    if !(path.ends_with("flac") | path.ends_with("mp3") | path.ends_with("ogg")) {
        return Err("Path requires an audio file extension")?;
    }

    let track_number = bytes.get(NUMBER_POS).ok_or("Invalid track")?;
    let disc_number = bytes.get(DISC_POS).ok_or("Invalid disc")?;

    let gain = f32::from_le_bytes(bytes.get(GAIN_POS).ok_or("Invalid gain")?.try_into()?);

    Ok(Song {
        title: title.to_string(),
        album: album.to_string(),
        artist: artist.to_string(),
        disc_number: *disc_number,
        track_number: *track_number,
        path: path.to_string(),
        gain,
    })
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
        // log!(
        //     "Warning: {} overflowed {} bytes! Metadata will be truncated.",
        //     path,
        //     SONG_LEN
        // );
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
    // let _ex = path.extension().unwrap();
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
}

pub type Database = BTreeMap<String, Vec<Album>>;

//Browser Queries:

///Get all aritist names.
pub fn artists(db: &Database) -> Vec<&String> {
    let mut v = Vec::from_iter(db.keys());
    v.sort_unstable_by_key(|artist| artist.to_ascii_lowercase());
    v
}

///Get all albums by an artist.
pub fn albums_by_artist<'a>(db: &'a Database, artist: &str) -> Option<&'a Vec<Album>> {
    db.get(artist)
}

///Get album by artist and album name.
pub fn album<'a>(db: &'a Database, artist: &str, album: &str) -> Option<&'a Album> {
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
pub fn song<'a>(
    db: &'a Database,
    artist: &str,
    album: &str,
    disc: u8,
    number: u8,
) -> Option<&'a Song> {
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
pub fn artist<'a>(db: &'a Database, artist: &str) -> Option<&'a Vec<Album>> {
    db.get(artist)
}

//Search Queries:

///Search the database and return the 25 most accurate matches.
pub fn search<'a>(db: &'a Database, query: &str) -> Vec<Item<'a>> {
    let query = query.to_lowercase();
    let results = RwLock::new(Vec::new());

    let iter = db.par_iter();

    iter.for_each(|(artist, albums)| {
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

const MIN_ACCURACY: f64 = 0.70;

pub fn jaro<'a>(query: &str, input: Item<'a>) -> Result<(Item<'a>, f64), (Item<'a>, f64)> {
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

pub fn read_database() -> Result<Database, Box<dyn Error + Send + Sync>> {
    let bytes = fs::read(database_path())?;
    let mut data: BTreeMap<String, Vec<Album>> = BTreeMap::new();

    let song_count = bytes.len() / SONG_LEN;

    if bytes.len() % SONG_LEN != 0 {
        return Err("Size of database is incorrect")?;
    }

    //Load all songs into memory.
    let songs: Vec<Song> = (0..song_count)
        .into_par_iter()
        .flat_map(|i| {
            let pos = i * SONG_LEN;
            let bytes = &bytes.get(pos..pos + SONG_LEN).ok_or("Invalid song")?;
            bytes_to_song(bytes)
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

    Ok(data as Database)
}

#[derive(Debug)]
pub enum ScanResult {
    Completed,
    CompletedWithErrors(Vec<String>),
    FileInUse,
}

pub fn create_database(path: impl ToString) -> JoinHandle<ScanResult> {
    let path = path.to_string();
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
                    .map(|dir| path_to_bytes(dir.path()))
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

                let _db = read_database().unwrap();

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

pub fn create_database_single(path: impl ToString) -> ScanResult {
    let path = path.to_string();
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
                .map(|dir| path_to_bytes(dir.path()))
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

            let _db = read_database().unwrap();

            if errors.is_empty() {
                ScanResult::Completed
            } else {
                ScanResult::CompletedWithErrors(errors)
            }
        }
        Err(_) => ScanResult::FileInUse,
    }
}

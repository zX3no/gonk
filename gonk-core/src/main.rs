#![allow(unused)]
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use gonk_core::db::Database;
use gonk_core::*;

// // #[repr(packed)]
// #[derive(Default, Debug)]
// pub struct Song {
//     pub text: Text,
//     pub number: u8,
//     pub disc: u8,
//     pub gain: f32,
//     pub padding: Vec<u8>,
// }

// // #[repr(packed)]
// #[derive(Default, Debug)]
// pub struct Text {
//     pub artist_len: u16,
//     pub album_len: u16,
//     pub title_len: u16,
//     pub path_len: u16,
//     pub artist: &'static str,
//     pub album: &'static str,
//     pub title: &'static str,
//     pub path: &'static str,
//     pub padding: Vec<u8>,
// }

// #[derive(Default, Debug)]
// pub struct S {
//     pub text: T,
//     pub number: u8,
//     pub disc: u8,
//     pub gain: f32,
// }

// impl S {
//     pub fn as_bytes(&self) -> Vec<u8> {
//         [
//             self.text.as_bytes().as_slice(),
//             &[self.number, self.disc],
//             self.gain.to_le_bytes().as_slice(),
//         ]
//         .concat()
//     }
// }

// #[derive(Default, Debug)]
// pub struct T {
//     pub artist: &'static str,
//     pub album: &'static str,
//     pub title: &'static str,
//     pub path: &'static str,
// }

// impl T {
//     pub fn as_bytes(&self) -> Vec<u8> {
//         [
//             (self.artist.len() as u16).to_le_bytes().as_slice(),
//             (self.album.len() as u16).to_le_bytes().as_slice(),
//             (self.title.len() as u16).to_le_bytes().as_slice(),
//             (self.path.len() as u16).to_le_bytes().as_slice(),
//             self.artist.as_bytes(),
//             self.album.as_bytes(),
//             self.title.as_bytes(),
//             self.path.as_bytes(),
//         ]
//         .concat()
//     }
// }

// pub const unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
//     ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
// }

// fn main() {
//     let song = Song {
//         text: Text {
//             artist_len: "artist".len() as u16,
//             album_len: "album".len() as u16,
//             title_len: "title".len() as u16,
//             path_len: "path".len() as u16,
//             artist: "artist",
//             album: "album",
//             title: "title",
//             path: "path",
//             padding: Vec::new(),
//         },
//         number: 1,
//         disc: 1,
//         gain: 1.0,
//         padding: Vec::new(),
//     };

//     let bytes: &[u8] = unsafe { any_as_u8_slice(&song) };
//     let pointer: *const [u8; std::mem::size_of::<Song>()] =
//         bytes as *const _ as *const [u8; std::mem::size_of::<Song>()];
//     let song: Song = unsafe { std::mem::transmute(*pointer) };

//     dbg!(song);
// }

/// A Chunk can hold a maximum of 4 songs.
/// However a single song can span an entire chunk.
/// I would like songs to span multiple chunks without a B-Tree.
///
/// There should be an identifier that says whether a chunk
/// is a part of a series of chunks.
///
/// Say Chunk 5/8
/// This tells the parsers to go back 4 chunks to chunk 1/8
/// and read 1-8.
///
/// How will data be stored across chunks?
/// If the name of an artist can be more than one chunk
/// where is it's length stored?
///
/// Should there be a max length of u16?
///
/// Perhaps multi-chunks should store the position of every element
/// in the first chunk
///
/// Chunk 1/3
/// position_1: 8
/// postition_2: 0
/// postition_3: 0
/// postition_4: 0
/// artist_len: 2324
/// album_len: 10
/// title_len: 32
/// path_len: 255
///
/// Artist...
///
/// Chunk 2/3
/// Artist ...
///
/// Chunk 3/3
/// Album
/// Title
/// Path
///
/// How large should a chunk be?
/// 64, 128, 256, 512?
// struct Chunk {
//     position_1: u16,
//     position_2: u16,
//     position_3: u16,
//     position_4: u16,
//     songs: Vec<S>,
// }

//TODO: Currently errors are not propergated in the database.
//I would like some kind of stacktrace to make it easier to hunt things down.
//
//I would also like safe version of all functions.
//
//Really none of the effort is worth it. I think a total re-write is in order.
//Worst of all, using a B-Tree to cache the songs wasn't the cause of the
//performance issue. I still have no idea really.

fn main() {
    // let settings = unsafe { Settings::new() };
    let song  = Song {
    title: "Interstellar (Original Motion Picture Soundtrack)  (Expanded Edition)".to_string(),
    album: "Interstellar (Original Motion Picture Soundtrack)  (Expanded Edition)".to_string(),
    artist: "Hans Zimmer".to_string(),
    disc_number: 1,
    track_number: 7,
    path: "D:/OneDrive/Music\\Hans Zimmer\\Interstellar (Original Motion Picture Soundtrack)  (Expanded Edition)\\07. Hans Zimmer - The Wormhole.flac".to_string(),
    gain: 0.8679606,
    };
    let b = song_to_bytes(
        &song.artist,
        &song.album,
        &song.title,
        &song.path,
        song.track_number,
        song.disc_number,
        song.gain,
    );
    let s = bytes_to_song(&b);
    let b2 = song_to_bytes(
        &s.artist,
        &s.album,
        &s.title,
        &s.path,
        s.track_number,
        s.disc_number,
        s.gain,
    );
    let s2 = bytes_to_song(&b2);
    assert_eq!(song, s);
    assert_eq!(s, s2);
}

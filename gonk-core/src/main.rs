#![allow(unused)]
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use gonk_core::{
    data::{Song, Text, S, T},
    db::{Album, Database},
};

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
struct Chunk {
    position_1: u16,
    position_2: u16,
    position_3: u16,
    position_4: u16,
    songs: Vec<S>,
}

fn main() {
    // let text = T {
    //     artist: "artist",
    //     album: "album",
    //     title: "title",
    //     path: "path",
    // };
    // let song = S {
    //     text,
    //     number: 10,
    //     disc: 1,
    //     gain: 1.0,
    // };
    // dbg!(song.as_bytes());
    // dbg!(song.as_bytes().len());
}

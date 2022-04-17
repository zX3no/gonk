use gonk_core::ServerConfig;
use gonk_core::{Index, Song};
use serde::{Deserialize, Serialize};
use static_init::dynamic;
pub use {client::Client, server::Server};

mod client;
mod server;

#[dynamic]
static CONFIG: ServerConfig = ServerConfig::new();

#[derive(Serialize, Deserialize, Debug)]
pub enum State {
    Playing,
    Paused,
    Stopped,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    AddPath(String),
    Add(Vec<u64>),
    PlayArtist(String),
    PlayIndex(usize),
    Delete(usize),
    ClearQueue,
    TogglePlayback,
    VolumeUp,
    VolumeDown,
    Prev,
    Next,
    SeekTo(f64),
    SeekBy(f64),
    ShutDown,
    Randomize,

    GetElapsed,
    GetState,
    GetVolume,
    GetQueue,
    GetBrowser,
    GetArtist(String),
    GetAlbum(String, String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Elapsed(f64),
    State(State),
    Volume(u16),
    Queue(Queue),
    Update(Update),
    Browser(Browser),
    Artist(Artist),
    Album(Vec<MinSong>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Browser {
    artists: Vec<String>,
    first_artist: Artist,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Artist {
    album_names: Vec<String>,
    selected_album: Vec<MinSong>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Queue {
    pub songs: Index<MinSong>,
    pub duration: f64,
}

//when the song changes instead of sending the entire queue
//again just send the new selected song
//durations aren't held in songs anymore so send that too.

//maybe just remove this, probably not faster and over complicated
#[derive(Serialize, Deserialize, Debug)]
pub struct Update {
    pub index: Option<usize>,
    pub duration: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinSong {
    pub number: u64,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub id: Option<u64>,
}

impl From<Song> for MinSong {
    fn from(song: Song) -> Self {
        Self {
            number: song.number,
            name: song.name,
            album: song.album,
            artist: song.artist,
            id: song.id,
        }
    }
}

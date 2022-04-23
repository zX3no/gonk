use crate::{MinSong, Request, Response, State};
use gonk_core::{Client as C, Config, Index, CLIENT_CONFIG};
use std::{
    io::{Read, Write},
    net::TcpStream,
};

pub struct Client {
    stream: TcpStream,
    pub queue: Index<MinSong>,
    pub state: State,
    pub volume: u16,
    pub elapsed: f64,
    pub duration: f64,

    pub artists: Index<String>,
    pub albums: Index<String>,
    pub songs: Index<MinSong>,
}

impl Client {
    pub fn new() -> Self {
        let config: Config<C> = Config::new(CLIENT_CONFIG.as_path());
        let stream =
            TcpStream::connect(config.data.server_ip).expect("Could not connect to server.");
        stream.set_nonblocking(true).unwrap();

        Self {
            stream,
            queue: Index::default(),
            state: State::Stopped,
            volume: 0,
            elapsed: 0.0,
            duration: 0.0,
            artists: Index::default(),
            albums: Index::default(),
            songs: Index::default(),
        }
    }
    pub fn update(&mut self) {
        let mut buf = [0u8; 4];
        if self.stream.read_exact(&mut buf[..]).is_ok() {
            //get the payload size
            let size = u32::from_le_bytes(buf);

            //read the payload
            let mut payload = vec![0; size as usize];
            self.stream.read_exact(&mut payload[..]).unwrap();
            let response: Response = bincode::deserialize(&payload).unwrap();
            match response {
                Response::Elapsed(e) => self.elapsed = e,
                Response::State(s) => self.state = s,
                Response::Volume(v) => self.volume = v,
                Response::Queue(q) => {
                    self.queue = q.songs;
                    self.duration = q.duration
                }
                Response::Update(uq) => {
                    self.duration = uq.duration;
                    self.queue.select(uq.index);
                }
                Response::Browser(b) => {
                    self.artists = Index::new(b.artists, Some(0));
                    self.albums = Index::new(b.first_artist.album_names, Some(0));
                    self.songs = Index::new(b.first_artist.selected_album, Some(0));
                }
                Response::Artist(a) => {
                    self.albums = Index::new(a.album_names, Some(0));
                    self.songs = Index::new(a.selected_album, Some(0));
                }
                Response::Album(songs) => self.songs = Index::new(songs, Some(0)),
            }
        }
    }
    fn send(&mut self, request: Request) {
        let encode = bincode::serialize(&request).unwrap();
        let size = encode.len() as u32;

        self.stream.write_all(&size.to_le_bytes()).unwrap();
        self.stream.write_all(&encode).unwrap();
    }
    pub fn volume_down(&mut self) {
        self.send(Request::VolumeDown);
        //HACK: this might get out of sync
        self.volume = self.volume.saturating_sub(5);
    }
    pub fn volume_up(&mut self) {
        self.send(Request::VolumeUp);
        //HACK: this might get out of sync
        let v = self.volume.saturating_add(5);
        if v > 100 {
            self.volume = 100;
        } else {
            self.volume = v;
        }
    }
    pub fn next(&mut self) {
        self.send(Request::Next);

        //HACK: this might get out of sync
        self.queue.down();
    }
    pub fn prev(&mut self) {
        self.send(Request::Prev);

        //HACK: this might get out of sync
        self.queue.up();
    }
    pub fn toggle_playback(&mut self) {
        self.send(Request::TogglePlayback);
    }
    pub fn add_ids(&mut self, ids: &[u64]) {
        self.send(Request::Add(ids.to_vec()));
    }
    pub fn clear_songs(&mut self) {
        self.send(Request::ClearQueue);
        //HACK: this might get out of sync
        self.queue = Index::default();
        self.state = State::Stopped;
    }
    pub fn seek_to(&mut self, pos: f64) {
        self.send(Request::SeekTo(pos))
    }
    pub fn seek_by(&mut self, amount: f64) {
        self.send(Request::SeekBy(amount))
    }
    pub fn delete_song(&mut self, id: usize) {
        self.send(Request::Delete(id));
    }
    pub fn randomize(&mut self) {
        self.send(Request::Randomize);
    }
    pub fn play_index(&mut self, i: usize) {
        self.send(Request::PlayIndex(i));
        //HACK: this might get out of sync
        self.queue.select(Some(i));
    }
    pub fn play_artist(&mut self, artist: String) {
        self.send(Request::PlayArtist(artist));
    }
    pub fn add_path(&mut self, path: String) {
        self.send(Request::AddPath(path));
    }
    pub fn update_artist(&mut self, artist: String) {
        self.send(Request::GetArtist(artist));
    }
    pub fn update_album(&mut self, album: String, artist: String) {
        self.send(Request::GetAlbum(album, artist));
    }
    #[must_use]
    pub fn sync(mut self) -> Self {
        self.send(Request::GetBrowser);
        self.send(Request::GetQueue);
        self.send(Request::GetVolume);
        self.send(Request::GetState);
        self
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

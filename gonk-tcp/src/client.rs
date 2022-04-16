use crate::{Event, Response, CONFIG};
use crossbeam_channel::{unbounded, Receiver};
use gonk_types::{Index, Song};
use std::{
    io::{Read, Write},
    net::TcpStream,
    thread,
};

pub struct Client {
    stream: TcpStream,
    receiver: Receiver<Response>,
    pub queue: Index<Song>,
    pub paused: bool,
    pub volume: u16,
    pub elapsed: f64,
    pub duration: f64,

    pub artists: Vec<String>,
    pub albums: Vec<String>,
    pub songs: Vec<(u64, String)>,
}

impl Client {
    pub fn new() -> Self {
        let stream = TcpStream::connect(CONFIG.ip()).expect("Could not connect to server.");
        let (sender, receiver) = unbounded();
        let mut s = stream.try_clone().unwrap();

        thread::spawn(move || {
            let mut buf = [0u8; 4];
            loop {
                if s.read_exact(&mut buf[..]).is_ok() {
                    //get the payload size
                    let size = u32::from_le_bytes(buf);

                    //read the payload
                    let mut payload = vec![0; size as usize];
                    s.read_exact(&mut payload[..]).unwrap();
                    let res: Response = bincode::deserialize(&payload).unwrap();
                    sender.send(res).unwrap();
                }
            }
        });

        Self {
            stream,
            receiver,
            queue: Index::default(),
            paused: false,
            volume: 0,
            elapsed: 0.0,
            duration: 0.0,
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
        }
    }
    pub fn update(&mut self) {
        if let Ok(response) = self.receiver.try_recv() {
            match response {
                Response::Elapsed(e) => self.elapsed = e,
                Response::Paused(p) => self.paused = p,
                Response::Volume(v) => self.volume = v,
                Response::Queue(q) => {
                    self.queue = q.songs;
                    self.duration = q.duration
                }
                Response::Update(uq) => {
                    self.duration = uq.duration;
                    self.queue.select(uq.index);
                }
                Response::Artists(a) => self.artists = a,
                Response::Artist(a) => {
                    self.albums = a.albums.iter().map(|album| album.name.clone()).collect();
                    self.songs = a
                        .albums
                        .first()
                        .unwrap()
                        .songs
                        .iter()
                        .map(|song| (song.number, song.name.clone()))
                        .collect();
                }
            }
        }
    }
    fn send(&mut self, event: Event) {
        let encode = bincode::serialize(&event).unwrap();
        let size = encode.len() as u32;

        self.stream.write_all(&size.to_le_bytes()).unwrap();
        self.stream.write_all(&encode).unwrap();
    }
    pub fn volume_down(&mut self) {
        self.send(Event::VolumeDown);
        //HACK: this might get out of sync
        self.volume = self.volume.saturating_sub(5);
    }
    pub fn volume_up(&mut self) {
        self.send(Event::VolumeUp);
        //HACK: this might get out of sync
        let v = self.volume.saturating_add(5);
        if v > 100 {
            self.volume = 100;
        } else {
            self.volume = v;
        }
    }
    pub fn next(&mut self) {
        self.send(Event::Next);

        //HACK: this might get out of sync
        self.queue.down();
    }
    pub fn prev(&mut self) {
        self.send(Event::Prev);

        //HACK: this might get out of sync
        self.queue.up();
    }
    pub fn toggle_playback(&mut self) {
        self.send(Event::TogglePlayback);

        //HACK: this might get out of sync
        self.paused = !self.paused;
    }
    pub fn add_ids(&mut self, ids: &[u64]) {
        self.send(Event::Add(ids.to_vec()));
    }
    pub fn clear_songs(&mut self) {
        self.send(Event::ClearQueue);
        //HACK: this might get out of sync
        self.queue = Index::default();
    }
    pub fn seek_to(&mut self, pos: f64) {
        self.send(Event::SeekTo(pos))
    }
    pub fn seek_by(&mut self, amount: f64) {
        self.send(Event::SeekBy(amount))
    }
    pub fn delete_song(&mut self, id: usize) {
        self.send(Event::Delete(id));
    }
    pub fn randomize(&mut self) {
        self.send(Event::Randomize);
    }
    pub fn play_index(&mut self, i: usize) {
        self.send(Event::PlayIndex(i));
        //HACK: this might get out of sync
        self.queue.select(Some(i));
    }
    pub fn add_path(&mut self, path: String) {
        self.send(Event::AddPath(path));
    }
    pub fn get_artist(&mut self, artist: String) {
        self.send(Event::GetArtist(artist));
    }
    // pub fn update_albums(&mut self, i: Option<usize>) {
    //     if let Some(i) = i {
    //         if let Some(artist) = self.artists.get(i).cloned() {
    //             self.send(Event::GetAlbums(artist));
    //         };
    //     }
    // }
    // pub fn update_songs(&mut self, album: Option<usize>, artist: Option<usize>) {
    //     //TODO: wtf?
    //     if let Some(album) = album {
    //         if let Some(artist) = artist {
    //             if let Some(artist) = self.artists.get(artist).cloned() {
    //                 if let Some(album) = self.albums.get(album).cloned() {
    //                     self.send(Event::GetSongs(album, artist));
    //                 };
    //             };
    //         }
    //     }
    // }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

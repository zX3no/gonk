#![allow(unused)]
use gonk_database::Database;
use gonk_types::{Index, Song};
use rodio::Player;
use serde::{Deserialize, Serialize};
use std::{
    collections::binary_heap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    thread,
};

pub struct Server {}

impl Server {
    pub fn run() {
        let listener = TcpListener::bind("localhost:3333").unwrap();
        let mut player = Player::new(10);
        let db = Database::new().unwrap();

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    Server::handle_client(stream, &mut player, db);
                    println!("Server shutting down.");
                    break;
                }
                Err(e) => println!("Server Error: {}", e),
            }
        }
    }
    fn handle_client(mut stream: TcpStream, mut player: &mut Player, db: Database) {
        let mut buf = [0u8; 4];
        loop {
            if player.elapsed() > player.duration {
                player.next_song();

                //update the clients current song
                Server::send(
                    &mut stream,
                    Response::Update(Update {
                        index: player.songs.index,
                        duration: player.duration,
                    }),
                );
            }

            if stream.read_exact(&mut buf[..]).is_ok() {
                //get the payload size
                let size = u32::from_le_bytes(buf);

                //read the payload
                let mut payload = vec![0; size as usize];
                stream.read_exact(&mut payload[..]).unwrap();

                let event = bincode::deserialize(&payload).unwrap();
                println!("Received: Event::{:?}", event);
                match event {
                    Request::ShutDown => break,
                    Request::Add(ids) => {
                        let songs = db.get_songs_from_id(&ids);
                        player.add_songs(songs);
                    }
                    Request::TogglePlayback => player.toggle_playback(),
                    Request::VolumeDown => {
                        player.volume_down();
                    }
                    Request::VolumeUp => {
                        player.volume_down();
                    }
                    Request::Prev => player.prev_song(),
                    Request::Next => player.next_song(),
                    Request::ClearSongs => player.clear_songs(),
                    Request::SeekBy(amount) => player.seek_by(amount),
                    Request::SeekTo(pos) => player.seek_to(pos),
                    Request::Delete(id) => player.delete_song(id),
                    Request::Randomize => player.randomize(),
                    Request::PlayIndex(i) => player.play_index(i),
                    Request::GetElapsed => {
                        Server::send(&mut stream, Response::Elapsed(player.elapsed()))
                    }
                    Request::GetPaused => {
                        Server::send(&mut stream, Response::Paused(player.is_paused()))
                    }
                    Request::GetVolume => {
                        Server::send(&mut stream, Response::Volume(player.volume))
                    }
                    _ => (),
                }
            }
        }
    }
    fn send(stream: &mut TcpStream, response: Response) {
        println!("Sent: Response::{:?}", response);
        let encode = bincode::serialize(&response).unwrap();
        let size = encode.len() as u32;

        stream.write_all(&size.to_le_bytes());
        stream.write_all(&encode);
    }
}

pub struct Client {
    stream: TcpStream,
    receiver: Receiver<Response>,
    pub queue: Index<Song>,
    pub paused: bool,
    pub volume: u16,
    pub elapsed: f64,
    pub duration: f64,
}

impl Client {
    pub fn new() -> Self {
        let stream = TcpStream::connect("localhost:3333").expect("Could not connect to server.");
        let (sender, receiver) = sync_channel(0);
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
        }
    }
    pub fn update(&mut self) {
        if let Ok(res) = self.receiver.try_recv() {
            match res {
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
            }
        }
    }
    pub fn send(&mut self, event: Request) {
        let encode = bincode::serialize(&event).unwrap();
        let size = encode.len() as u32;

        self.stream.write_all(&size.to_le_bytes()).unwrap();
        self.stream.write_all(&encode).unwrap();
    }
    pub fn volume_down(&mut self) {
        self.send(Request::VolumeDown);
    }
    pub fn volume_up(&mut self) {
        self.send(Request::VolumeUp);
    }
    pub fn next(&mut self) {
        self.send(Request::Next);
    }
    pub fn prev(&mut self) {
        self.send(Request::Prev);
    }
    pub fn toggle_playback(&mut self) {
        self.send(Request::TogglePlayback);
    }
    pub fn add_ids(&mut self, ids: &[u64]) {
        self.send(Request::Add(ids.to_vec()));
    }
    pub fn clear_songs(&mut self) {
        self.send(Request::ClearSongs);
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
    }
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Add(Vec<u64>),
    PlayIndex(usize),
    Delete(usize),
    ClearSongs,
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
    GetPaused,
    GetVolume,
    GetQueue,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Elapsed(f64),
    Paused(bool),
    Volume(u16),
    Queue(Queue),
    Update(Update),
}

//send which songs are in the queue
//also send the duration of the current song
#[derive(Serialize, Deserialize, Debug)]
pub struct Queue {
    pub songs: Index<Song>,
    pub duration: f64,
}

//when the song changes instead of sending the entire queue
//again just send the new index to select
//durations aren't held in songs anymore so send that too.
#[derive(Serialize, Deserialize, Debug)]
pub struct Update {
    pub index: Option<usize>,
    pub duration: f64,
}

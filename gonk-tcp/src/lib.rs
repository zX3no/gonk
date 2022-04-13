#![allow(unused)]
use gonk_database::Database;
use gonk_types::{Index, Song};
use rodio::Player;
use serde::{Deserialize, Serialize};
use std::{
    collections::binary_heap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
};

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn new() -> Self {
        let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
        println!("Server listening on port 3333");
        Self { listener }
    }
    pub fn run(&mut self) {
        let mut player = Player::new(10);
        let db = Database::new().unwrap();

        for stream in self.listener.incoming() {
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
                    Response::UpdateQueue(UpdateQueue {
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
        let encode = bincode::serialize(&response).unwrap();
        let size = encode.len() as u32;

        stream.write_all(&size.to_le_bytes());
        stream.write_all(&encode);
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Client {
    //TODO: this tcp stream should be removed
    //Client.run() will consume self
    //so you won't be able to do Client.next()
    //since the tcpstream was also consumed
    //the connection should be established
    //somewhere else, might help with the slow connection time.
    stream: TcpStream,

    pub queue: Index<Song>,
    pub paused: bool,
    pub volume: u16,
    pub elapsed: f64,
    pub duration: f64,
}

impl Client {
    pub fn new() -> Self {
        match TcpStream::connect("localhost:3333") {
            Ok(mut stream) => {
                println!("Successfully connected to server in port 3333");
                Self {
                    stream,
                    ..Default::default()
                }
            }
            Err(e) => panic!("Failed to connect: {}", e),
        }
    }
    //TODO: should run pass in an arc<rwlock<client>> instead??
    //i don't think it should be part of the struct
    pub fn run(mut self) {
        //TODO: loop and wait for messages from the server
        //after receiving a response update the client data
        //this should be done on another thread so data is blocked
        thread::spawn(move || {
            let mut buf = [0u8; 4];
            loop {
                if self.stream.read_exact(&mut buf[..]).is_ok() {
                    //get the payload size
                    let size = u32::from_le_bytes(buf);

                    //read the payload
                    let mut payload = vec![0; size as usize];
                    self.stream.read_exact(&mut payload[..]).unwrap();

                    let res: Response = bincode::deserialize(&payload).unwrap();
                    match res {
                        Response::Elapsed(elapsed) => self.elapsed = elapsed,
                        Response::Paused(paused) => self.paused = paused,
                        Response::Volume(volume) => self.volume = volume,
                        Response::Queue(queue) => {
                            self.queue = queue.songs;
                            if let Some(duration) = queue.duration {
                                self.duration = duration
                            }
                        }
                        Response::UpdateQueue(uq) => {
                            self.duration = uq.duration;
                            self.queue.select(uq.index);
                        }
                    }
                }
            }
        });
    }
    fn send(&mut self, event: Request) {
        let encode = bincode::serialize(&event).unwrap();
        let size = encode.len() as u32;

        self.stream.write_all(&size.to_le_bytes());
        self.stream.write_all(&encode);
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
    pub fn is_paused(&mut self) -> bool {
        self.paused
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
    UpdateQueue(UpdateQueue),
}

//send which songs are in the queue
//also send the duration of the current song
#[derive(Serialize, Deserialize, Debug)]
pub struct Queue {
    songs: Index<Song>,
    duration: Option<f64>,
}

//when the song changes instead of sending the entire queue
//again just send the new index to select
//durations aren't held in songs anymore so send that too.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateQueue {
    index: Option<usize>,
    duration: f64,
}

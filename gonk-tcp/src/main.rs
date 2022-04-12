#![allow(unused)]
use gonk_database::Database;
use rodio::Player;
use serde::{Deserialize, Serialize};
use std::{
    collections::binary_heap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

struct Server {
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
            player.update();

            if stream.read_exact(&mut buf[..]).is_ok() {
                //get the payload size
                let size = u32::from_le_bytes(buf);

                //read the payload
                let mut payload = vec![0; size as usize];
                stream.read_exact(&mut payload[..]).unwrap();

                match bincode::deserialize(&payload).unwrap() {
                    Event::ShutDown => break,
                    Event::Add(ids) => {
                        let songs = db.get_songs_from_id(&ids);
                        player.add_songs(songs);
                    }
                    Event::TogglePlayback => player.toggle_playback(),
                    Event::VolumeDown => {
                        player.volume_down();
                    }
                    Event::VolumeUp => {
                        player.volume_down();
                    }
                    Event::Prev => player.prev_song(),
                    Event::Next => player.next_song(),
                    _ => (),
                }
            }
        }
    }
}

struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new() -> Self {
        match TcpStream::connect("localhost:3333") {
            Ok(mut stream) => {
                println!("Successfully connected to server in port 3333");
                Self { stream }
            }
            Err(e) => panic!("Failed to connect: {}", e),
        }
    }
    pub fn send(&mut self, event: Event) {
        let encode = bincode::serialize(&event).unwrap();
        let size = encode.len() as u32;

        self.stream.write_all(&size.to_le_bytes());
        self.stream.write_all(&encode);
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Event {
    Add(Vec<usize>),
    TogglePlayback,
    VolumeUp,
    VolumeDown,
    Prev,
    Next,
    ShutDown,
}

fn main() {
    let handle = thread::spawn(|| Server::new().run());

    let mut c = Client::new();

    c.send(Event::Add(vec![1, 2, 3]));
    c.send(Event::Next);
    c.send(Event::TogglePlayback);
    c.send(Event::TogglePlayback);

    handle.join();
}

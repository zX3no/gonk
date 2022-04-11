#![allow(unused)]
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

        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    Server::handle_client(stream, &mut player);
                    println!("Server shutting down.");
                    break;
                }
                Err(e) => println!("Server Error: {}", e),
            }
        }
    }
    fn handle_client(mut stream: TcpStream, mut player: &mut Player) {
        let mut buf = [0_u8; 16];
        let mut buffer = [0u8; 4];
        loop {
            if stream.read_exact(&mut buffer[..]).is_ok() {
                //get the payload size
                let size = u32::from_le_bytes(buffer);
                println!("Receive size: {}", size);

                //read the payload
                let mut payload = vec![0; size as usize];
                stream.read_exact(&mut payload[..]).unwrap();

                let e: Event = bincode::deserialize(&payload).unwrap();
                match e {
                    Event::Shutdown => break,
                    Event::PlaySong(id) => println!("Playing song: {}", id),
                    Event::VolumeDown => {
                        player.volume_down();
                    }
                    Event::VolumeUp => {
                        player.volume_down();
                    }
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
    pub fn run(&mut self) {
        let e = Event::Shutdown;
        let encode = bincode::serialize(&e).unwrap();
        let size = encode.len() as u32;
        println!("Send size: {}", size);
        self.stream.write_all(&size.to_le_bytes());
        self.stream.write_all(&encode);
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Event {
    PlaySong(u64),
    VolumeUp,
    VolumeDown,
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]

struct Command {
    event: Event,
}

fn main() {
    let handle = thread::spawn(|| Server::new().run());
    Client::new().run();
    handle.join();
}

use crossbeam_channel::{unbounded, Receiver, Sender};
use gonk_database::Database;
use gonk_types::{Index, Song};
use rodio::Player;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, sync_channel},
    thread,
};

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Add(Vec<u64>),
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

    Elapsed(f64),
    Paused(bool),
    Volume(u16),
    Queue(Queue),
    Update(Update),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    GetElapsed,
    GetPaused,
    GetVolume,
    GetQueue,
}

pub struct Server {
    listener: TcpListener,
    event_s: Sender<Event>,
    event_r: Receiver<Event>,
    request_s: Sender<Request>,
}

impl Server {
    pub fn new() -> Self {
        let listener = TcpListener::bind("localhost:3333").unwrap();
        let (request_s, request_r) = unbounded();
        let (event_s, event_r) = unbounded();
        let (es, er) = (event_s.clone(), event_r.clone());

        thread::spawn(|| Server::player_loop(es, er, request_r));

        Self {
            listener,
            event_s,
            event_r,
            request_s,
        }
    }
    fn player_loop(es: Sender<Event>, er: Receiver<Event>, rr: Receiver<Request>) {
        let mut player = Player::new(5);
        let db = Database::new().unwrap();

        loop {
            if player.elapsed() > player.duration {
                player.next_song();
                es.send(Event::Update(Update {
                    index: player.songs.index,
                    duration: player.duration,
                }))
                .unwrap();
            }

            //check if client wants to change player
            if let Ok(event) = er.try_recv() {
                println!("Event received: {:?}", event);
                match event {
                    Event::ShutDown => break,
                    Event::Add(ids) => {
                        let songs = db.get_songs_from_id(&ids);
                        player.add_songs(songs);

                        es.send(Event::Queue(Queue {
                            songs: player.songs.clone(),
                            duration: player.duration,
                        }))
                        .unwrap();
                    }
                    Event::TogglePlayback => player.toggle_playback(),
                    Event::VolumeDown => {
                        player.volume_down();
                        println!("Volume: {}", player.volume);
                    }
                    Event::VolumeUp => {
                        player.volume_up();
                        println!("Volume: {}", player.volume);
                    }
                    Event::Prev => player.prev_song(),
                    Event::Next => player.next_song(),
                    Event::ClearQueue => player.clear_songs(),
                    Event::SeekBy(amount) => player.seek_by(amount),
                    Event::SeekTo(pos) => player.seek_to(pos),
                    Event::Delete(id) => player.delete_song(id),
                    Event::Randomize => player.randomize(),
                    Event::PlayIndex(i) => player.play_index(i),
                    _ => (),
                }
            }

            //check if data should be sent to client
            if let Ok(request) = rr.try_recv() {
                println!("Request received: {:?}", request);
                match request {
                    Request::GetElapsed => es.send(Event::Elapsed(player.elapsed())).unwrap(),
                    Request::GetPaused => es.send(Event::Paused(player.is_paused())).unwrap(),
                    Request::GetVolume => es.send(Event::Volume(player.volume)).unwrap(),
                    Request::GetQueue => es
                        .send(Event::Queue(Queue {
                            songs: player.songs.clone(),
                            duration: player.duration,
                        }))
                        .unwrap(),
                };
            }
        }
    }
    pub fn run(&mut self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    let stream = stream.try_clone().unwrap();
                    let es = self.event_s.clone();
                    let er = self.event_r.clone();
                    let rs = self.request_s.clone();
                    thread::spawn(|| Server::handle_client(stream, es, er, rs));
                }
                Err(e) => println!("Server Error: {}", e),
            }
        }
        println!("Server shutting down.");
    }
    fn handle_client(
        mut stream: TcpStream,
        es: Sender<Event>,
        er: Receiver<Event>,
        rs: Sender<Request>,
    ) {
        //update the clients data on connect
        rs.send(Request::GetVolume).unwrap();
        rs.send(Request::GetQueue).unwrap();
        rs.send(Request::GetElapsed).unwrap();
        rs.send(Request::GetPaused).unwrap();

        let mut buf = [0u8; 4];
        loop {
            //send info about the player
            if let Ok(event) = er.try_recv() {
                println!("Local messge: {:?}", event);
                match event {
                    Event::Elapsed(_)
                    | Event::Paused(_)
                    | Event::Volume(_)
                    | Event::Queue(_)
                    | Event::Update(_) => {
                        println!("Sent: Response::{:?}", event);
                        let encode = bincode::serialize(&event).unwrap();
                        let size = encode.len() as u32;

                        stream.write_all(&size.to_le_bytes()).unwrap();
                        stream.write_all(&encode).unwrap();
                    }
                    //resend the event if we weren't meant to have it
                    _ => es.send(event).unwrap(),
                }
            }

            match stream.read_exact(&mut buf[..]) {
                Ok(_) => {
                    //get the payload size
                    let size = u32::from_le_bytes(buf);

                    //read the payload
                    let mut payload = vec![0; size as usize];
                    stream.read_exact(&mut payload[..]).unwrap();

                    let request = bincode::deserialize(&payload).unwrap();
                    println!("Server received: {:?}", request);
                    es.send(request).unwrap();
                }
                Err(e) => return println!("{}", e),
            }
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Queue {
    pub songs: Index<Song>,
    pub duration: f64,
}

//when the song changes instead of sending the entire queue
//again just send the new index to select
//durations aren't held in songs anymore so send that too.

//maybe just remove this, probably not faster and over complicated
#[derive(Serialize, Deserialize, Debug)]
pub struct Update {
    pub index: Option<usize>,
    pub duration: f64,
}

pub struct Client {
    stream: TcpStream,
    receiver: mpsc::Receiver<Event>,
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

                    let res: Event = bincode::deserialize(&payload).unwrap();
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
        if let Ok(event) = self.receiver.try_recv() {
            match event {
                Event::Elapsed(e) => self.elapsed = e,
                Event::Paused(p) => self.paused = p,
                Event::Volume(v) => self.volume = v,
                Event::Queue(q) => {
                    self.queue = q.songs;
                    self.duration = q.duration
                }
                Event::Update(uq) => {
                    self.duration = uq.duration;
                    self.queue.select(uq.index);
                }
                _ => (),
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
    }
    pub fn volume_up(&mut self) {
        self.send(Event::VolumeUp);
    }
    pub fn next(&mut self) {
        self.send(Event::Next);
    }
    pub fn prev(&mut self) {
        self.send(Event::Prev);
    }
    pub fn toggle_playback(&mut self) {
        self.send(Event::TogglePlayback);
    }
    pub fn add_ids(&mut self, ids: &[u64]) {
        self.send(Event::Add(ids.to_vec()));
    }
    pub fn clear_songs(&mut self) {
        self.send(Event::ClearQueue);
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
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

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

pub struct Server {
    listener: TcpListener,
    event_s: Sender<Event>,
    request_r: Receiver<Response>,
}

impl Server {
    pub fn new() -> Self {
        let listener = TcpListener::bind("localhost:0673").unwrap();
        let (request_s, request_r) = unbounded();
        let (event_s, event_r) = unbounded();

        thread::spawn(|| Server::player_loop(event_r, request_s));

        Self {
            listener,
            event_s,
            request_r,
        }
    }
    fn player_loop(er: Receiver<Event>, rs: Sender<Response>) {
        let mut player = Player::new(0);
        let db = Database::new().unwrap();

        let queue = |player: &Player| {
            rs.send(Response::Queue(Queue {
                songs: player.songs.clone(),
                duration: player.duration,
            }))
            .unwrap();
        };

        // let update = |player: &Player| {
        //     rs.send(Response::Update(Update {
        //         index: player.songs.index,
        //         duration: player.duration,
        //     }))
        //     .unwrap();
        // };

        let mut elapsed = 0.0;
        loop {
            if player.elapsed() > player.duration {
                player.next_song();
                queue(&player);
            }

            //send the position of the player
            //rounding is an optimisation to update every half a second.
            let e = player.elapsed().round();
            if elapsed != e {
                elapsed = e;
                rs.send(Response::Elapsed(player.elapsed())).unwrap();
            }

            //check if client wants to change player
            if let Ok(event) = er.try_recv() {
                println!("Event received: {:?}", event);
                match event {
                    Event::ShutDown => break,
                    Event::Add(ids) => {
                        let songs = db.get_songs_from_id(&ids);
                        player.add_songs(songs);

                        queue(&player);
                    }
                    Event::TogglePlayback => {
                        player.toggle_playback();
                        // rs.send(Response::Paused(player.is_paused())).unwrap();
                    }
                    Event::VolumeDown => {
                        player.volume_down();
                        // rs.send(Response::Volume(player.volume)).unwrap();
                    }
                    Event::VolumeUp => {
                        player.volume_up();
                        // rs.send(Response::Volume(player.volume)).unwrap();
                    }
                    Event::Prev => {
                        player.prev_song();
                        // update(&player);
                    }
                    Event::Next => {
                        player.next_song();
                        // update(&player);
                    }
                    Event::ClearQueue => {
                        player.clear_songs();
                        // queue(&player);
                    }
                    Event::SeekBy(amount) => player.seek_by(amount),
                    Event::SeekTo(pos) => player.seek_to(pos),
                    Event::Delete(id) => {
                        player.delete_song(id);
                        queue(&player);
                    }
                    Event::Randomize => {
                        player.randomize();
                        queue(&player);
                    }
                    Event::PlayIndex(i) => {
                        player.play_index(i);
                        // update(&player);
                    }
                    Event::GetElapsed => rs.send(Response::Elapsed(player.elapsed())).unwrap(),
                    Event::GetPaused => rs.send(Response::Paused(player.is_paused())).unwrap(),
                    Event::GetVolume => rs.send(Response::Volume(player.volume)).unwrap(),
                    Event::GetQueue => queue(&player),
                }
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
                    let rr = self.request_r.clone();
                    thread::spawn(|| Server::handle_client(stream, es, rr));
                }
                Err(e) => println!("Server Error: {}", e),
            }
        }
        println!("Server shutting down.");
    }
    fn handle_client(mut stream: TcpStream, es: Sender<Event>, rr: Receiver<Response>) {
        //update the clients data on connect
        es.send(Event::GetPaused).unwrap();
        es.send(Event::GetVolume).unwrap();
        es.send(Event::GetQueue).unwrap();

        let mut s = stream.try_clone().unwrap();
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 4];
            loop {
                //read_exact is blocking(i think)
                match s.read_exact(&mut buf[..]) {
                    Ok(_) => {
                        //get the payload size
                        let size = u32::from_le_bytes(buf);

                        //read the payload
                        let mut payload = vec![0; size as usize];
                        s.read_exact(&mut payload[..]).unwrap();

                        let request = bincode::deserialize(&payload).unwrap();
                        println!("Server received: {:?}", request);
                        es.send(request).unwrap();
                    }
                    Err(e) => return println!("{}", e),
                }
            }
        });

        loop {
            //quit when the client disconnects
            if handle.is_finished() {
                return;
            }

            if let Ok(response) = rr.try_recv() {
                println!("Received Response::{:?}", response);
                let encode = bincode::serialize(&response).unwrap();
                let size = encode.len() as u32;

                stream.write_all(&size.to_le_bytes()).unwrap();
                stream.write_all(&encode).unwrap();
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
//again just send the new selected song
//durations aren't held in songs anymore so send that too.

//maybe just remove this, probably not faster and over complicated
#[derive(Serialize, Deserialize, Debug)]
pub struct Update {
    pub index: Option<usize>,
    pub duration: f64,
}

pub struct Client {
    stream: TcpStream,
    receiver: mpsc::Receiver<Response>,
    pub queue: Index<Song>,
    pub paused: bool,
    pub volume: u16,
    pub elapsed: f64,
    pub duration: f64,
}

impl Client {
    pub fn new() -> Self {
        let stream = TcpStream::connect("localhost:0673").expect("Could not connect to server.");
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
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

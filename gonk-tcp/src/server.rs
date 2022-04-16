use crate::{Album, Artist, Event, Queue, Response, Update, CONFIG};
use crossbeam_channel::{unbounded, Receiver, Sender};
use gonk_database::{Database, ServerConfig};
use rodio::Player;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
};

pub struct Server {}

impl Server {
    pub fn run() {
        let listener = TcpListener::bind(CONFIG.ip()).unwrap();
        let (request_s, request_r) = unbounded();
        let (event_s, event_r) = unbounded();

        thread::spawn(|| Server::player_loop(event_r, request_s));

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    let stream = stream.try_clone().unwrap();
                    let es = event_s.clone();
                    let rr = request_r.clone();

                    thread::spawn(|| Server::handle_client(stream, es, rr));
                }
                Err(e) => println!("Server Error: {}", e),
            }
        }
        println!("Server shutting down.");
    }
    fn player_loop(er: Receiver<Event>, rs: Sender<Response>) {
        let mut player = Player::new(0);
        let db = Database::new().unwrap();
        //sync the datbase
        db.sync_database(&CONFIG.paths);

        let mut config = ServerConfig::new();

        let queue = |player: &Player| {
            let mut songs = player.songs.clone();

            //HACK: remove the path so there's less data to send
            for song in &mut songs.data {
                song.path = PathBuf::default();
            }

            rs.send(Response::Queue(Queue {
                songs,
                duration: player.duration,
            }))
            .unwrap();
        };

        let update = |player: &Player| {
            rs.send(Response::Update(Update {
                index: player.songs.index,
                duration: player.duration,
            }))
            .unwrap();
        };

        let artist = |artist: String| {
            let albums = db.get_all_albums_by_artist(&artist);

            let albums: Vec<Album> = albums
                .into_iter()
                .map(|album| Album {
                    songs: db
                        .get_songs_from_album(&album, &artist)
                        .into_iter()
                        .map(|song| song.nuke_useless())
                        .collect(),
                    name: album,
                })
                .collect();

            let artist = Artist {
                name: artist,
                albums,
            };

            rs.send(Response::Artist(artist)).unwrap();
        };

        let mut elapsed = 0.0;
        loop {
            if player.elapsed() > player.duration {
                player.next_song();
                update(&player);
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
                match event {
                    Event::ShutDown => break,
                    Event::AddPath(path) => {
                        if Path::new(&path).exists() {
                            println!("Adding path: {path}");
                            config.add_path(path.clone());
                            db.add_paths(&[path]);
                        }
                    }
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
                    Event::SeekBy(amount) => {
                        if player.seek_by(amount) {
                            update(&player);
                        }
                    }
                    Event::SeekTo(pos) => {
                        if player.seek_to(pos) {
                            update(&player);
                        }
                    }
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
                    Event::GetAllArtists => {
                        let artists = db.get_all_artists();
                        rs.send(Response::Artists(artists)).unwrap();
                    }
                    Event::GetFirstArtist => {
                        if let Some(a) = db.get_all_artists().first() {
                            artist(a.clone())
                        }
                    }
                    Event::GetArtist(a) => artist(a),
                }
            }
        }
    }

    fn handle_client(mut stream: TcpStream, es: Sender<Event>, rr: Receiver<Response>) {
        //update the clients data on connect
        es.send(Event::GetPaused).unwrap();
        es.send(Event::GetVolume).unwrap();
        es.send(Event::GetQueue).unwrap();
        es.send(Event::GetFirstArtist).unwrap();
        es.send(Event::GetAllArtists).unwrap();

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
                let encode = bincode::serialize(&response).unwrap();
                let size = encode.len() as u32;
                stream.write_all(&size.to_le_bytes()).unwrap();
                stream.write_all(&encode).unwrap();

                println!("Server sent: {:?}", response);
            }
        }
    }
}

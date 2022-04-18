use crate::{Artist, Browser, Event, MinSong, Queue, Response, State, Update, CONFIG};
use crossbeam_channel::{unbounded, Receiver, Sender};
use gonk_core::Index;
use gonk_core::{Database, ServerConfig};
use rodio::Player;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    thread,
    time::Duration,
};

pub struct Server {}

impl Server {
    pub fn run() {
        let listener = TcpListener::bind(CONFIG.ip()).unwrap();
        println!("Server running @ {}", CONFIG.ip());

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
    fn write(mut stream: &TcpStream, response: Response) {
        let encode = bincode::serialize(&response).unwrap();
        let size = encode.len() as u32;
        stream.write_all(&size.to_le_bytes()).unwrap();
        stream.write_all(&encode).unwrap();

        let response = match response {
            Response::Elapsed(_)
            | Response::State(_)
            | Response::Volume(_)
            | Response::Update(_) => format!("{:?}", response),
            Response::Queue(_) => String::from("Queue"),
            Response::Browser(_) => String::from("Browser"),
            Response::Artist(_) => String::from("Artist"),
            Response::Album(_) => String::from("Album"),
        };

        println!("{}: Response::{}", stream.peer_addr().unwrap(), response);
    }
    fn player_loop(er: Receiver<(TcpStream, Event)>, rs: Sender<Response>) {
        let mut player = Player::new(0);
        let db = Database::new().unwrap();
        //sync the datbase
        db.sync_database(&CONFIG.paths);

        let mut config = ServerConfig::new();

        let queue = |player: &Player| -> Response {
            let queue = player.songs.clone();
            let index = queue.index;
            let songs = queue.data.into_iter().map(MinSong::from).collect();

            Response::Queue(Queue {
                songs: Index::new(songs, index),
                duration: player.duration,
            })
        };

        let update = |player: &Player| -> Response {
            Response::Update(Update {
                index: player.songs.index,
                duration: player.duration,
            })
        };

        let artist = |artist: String| -> Artist {
            let albums = db.get_all_albums_by_artist(&artist);

            let album = albums.first().unwrap();
            let songs = db
                .get_songs_from_album(album, &artist)
                .into_iter()
                .map(MinSong::from)
                .collect();

            Artist {
                album_names: albums,
                selected_album: songs,
            }
        };

        let state = |player: &Player| -> Response {
            let state = if player.songs.is_empty() {
                State::Stopped
            } else if player.is_paused() {
                State::Paused
            } else {
                State::Playing
            };

            Response::State(state)
        };

        let mut old_elapsed = 0.0;
        loop {
            let elapsed = player.elapsed();
            if elapsed > player.duration {
                player.next_song();
                rs.send(update(&player)).unwrap();
            }

            //send the position of the player
            //rounding is an optimisation to update every second.
            let trunc = elapsed.trunc();
            if old_elapsed != trunc {
                old_elapsed = trunc;
                rs.send(Response::Elapsed(elapsed)).unwrap();
            }

            //if this isn't semi-blocking it will waste cpu cycles
            //16ms is probablby super over kill could change to 200ms.
            if let Ok((stream, event)) = er.recv_timeout(Duration::from_millis(16)) {
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

                        Server::write(&stream, queue(&player));
                        Server::write(&stream, state(&player));
                    }
                    Event::TogglePlayback => {
                        player.toggle_playback();
                        Server::write(&stream, state(&player));
                    }
                    Event::VolumeDown => {
                        player.volume_down();
                        //HACK: no response
                    }
                    Event::VolumeUp => {
                        player.volume_up();
                        //HACK: no response
                    }
                    Event::Prev => {
                        player.prev_song();
                        //HACK: no response
                    }
                    Event::Next => {
                        player.next_song();
                        //HACK: no response
                    }
                    Event::ClearQueue => {
                        player.clear_songs();
                        //HACK: no response
                    }
                    Event::SeekBy(amount) => {
                        if player.seek_by(amount) {
                            Server::write(&stream, update(&player));
                        }
                    }
                    Event::SeekTo(pos) => {
                        if player.seek_to(pos) {
                            Server::write(&stream, update(&player));
                        }
                    }
                    Event::Delete(id) => {
                        player.delete_song(id);
                        Server::write(&stream, queue(&player));
                    }
                    Event::Randomize => {
                        player.randomize();
                        Server::write(&stream, queue(&player));
                    }
                    Event::PlayIndex(i) => {
                        player.play_index(i);

                        Server::write(&stream, update(&player));
                        Server::write(&stream, state(&player));
                    }
                    Event::GetElapsed => {
                        Server::write(&stream, Response::Elapsed(player.elapsed()))
                    }
                    Event::GetState => {
                        Server::write(&stream, state(&player));
                    }
                    Event::GetVolume => Server::write(&stream, Response::Volume(player.volume)),
                    Event::GetQueue => Server::write(&stream, queue(&player)),
                    Event::GetBrowser => {
                        let artists = db.get_all_artists();
                        if let Some(a) = db.get_all_artists().first() {
                            let first_artist = artist(a.clone());
                            let browser = Browser {
                                artists,
                                first_artist,
                            };

                            Server::write(&stream, Response::Browser(browser));
                        }
                    }
                    Event::GetArtist(a) => {
                        let artist = artist(a);
                        Server::write(&stream, Response::Artist(artist));
                    }
                    Event::GetAlbum(album, artist) => {
                        let songs = db
                            .get_songs_from_album(&album, &artist)
                            .into_iter()
                            .map(MinSong::from)
                            .collect();

                        Server::write(&stream, Response::Album(songs));
                    }
                    Event::PlayArtist(artist) => {
                        let songs = db.get_songs_by_artist(&artist);
                        player.add_songs(songs);

                        Server::write(&stream, queue(&player));
                        Server::write(&stream, state(&player));
                    }
                }
            }
        }
    }
    fn handle_client(stream: TcpStream, es: Sender<(TcpStream, Event)>, rr: Receiver<Response>) {
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

                        let request: Event = bincode::deserialize(&payload).unwrap();
                        let r = request.clone();
                        es.send((s.try_clone().unwrap(), request)).unwrap();
                        println!("{}: Event::{}", s.peer_addr().unwrap(), r);
                    }
                    Err(e) => return println!("{}", e),
                }
            }
        });

        loop {
            if let Ok(response) = rr.recv() {
                //quit when client disconnects
                //keep in mind if no events are sent
                //this won't be checked
                if handle.is_finished() {
                    return;
                }

                Server::write(&stream, response);
            }
        }
    }
}

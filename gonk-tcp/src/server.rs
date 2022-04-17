use crate::{Artist, Browser, Event, Queue, Response, Update, CONFIG};
use crossbeam_channel::{unbounded, Receiver, Sender};
use gonk_database::{Database, ServerConfig};
use rodio::Player;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::Duration,
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

        let artist = |artist: String| -> Artist {
            let albums = db.get_all_albums_by_artist(&artist);

            let album = albums.first().unwrap();
            let songs = db
                .get_songs_from_album(album, &artist)
                .into_iter()
                .map(|song| song.nuke_useless())
                .collect();

            Artist {
                album_names: albums,
                selected_album: songs,
            }
        };

        let mut old_elapsed = 0.0;
        loop {
            let elapsed = player.elapsed();
            if elapsed > player.duration {
                player.next_song();
                update(&player);
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
            if let Ok(event) = er.recv_timeout(Duration::from_millis(16)) {
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
                    Event::GetBrowser => {
                        let artists = db.get_all_artists();
                        if let Some(a) = db.get_all_artists().first() {
                            let first_artist = artist(a.clone());
                            let browser = Browser {
                                artists,
                                first_artist,
                            };
                            rs.send(Response::Browser(browser)).unwrap();
                        }
                    }
                    Event::GetArtist(a) => {
                        let artist = artist(a);
                        rs.send(Response::Artist(artist)).unwrap();
                    }
                    Event::GetAlbum(album, artist) => {
                        let songs = db
                            .get_songs_from_album(&album, &artist)
                            .into_iter()
                            .map(|song| song.nuke_useless())
                            .collect();

                        rs.send(Response::Album(songs)).unwrap();
                    }
                    Event::PlayArtist(artist) => {
                        let songs = db.get_songs_by_artist(&artist);
                        player.add_songs(songs);

                        queue(&player);
                    }
                }
            }
        }
    }

    fn handle_client(mut stream: TcpStream, es: Sender<Event>, rr: Receiver<Response>) {
        //update the clients data on connect
        es.send(Event::GetPaused).unwrap();
        es.send(Event::GetVolume).unwrap();
        es.send(Event::GetQueue).unwrap();
        es.send(Event::GetBrowser).unwrap();

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
            } else if let Ok(response) = rr.recv() {
                let encode = bincode::serialize(&response).unwrap();
                let size = encode.len() as u32;
                stream.write_all(&size.to_le_bytes()).unwrap();
                stream.write_all(&encode).unwrap();

                println!("Server sent: {:?}", response);
            }
        }
    }
}

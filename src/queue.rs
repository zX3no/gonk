use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc, RwLock},
    thread,
    time::Duration,
};

use crate::player::Player;

pub struct Queue {
    pub songs: Arc<RwLock<Vec<(String, PathBuf)>>>,
    pub playing: Arc<RwLock<Option<usize>>>,
    pub player: Arc<RwLock<Player>>,
    pub selection: usize,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            songs: Arc::new(RwLock::new(Vec::new())),
            playing: Arc::new(RwLock::new(None)),
            player: Arc::new(RwLock::new(Player::new())),
            selection: 0,
        }
    }
    pub fn add(&mut self, song: String, path: &Path) {
        self.songs.write().unwrap().push((song, path.to_path_buf()));
    }
    pub fn remove(&mut self, index: usize) {
        self.songs.write().unwrap().remove(index);
    }
    pub fn run(&self) {
        let songs = self.songs.clone();
        let playing = self.playing.clone();
        let player = self.player.clone();

        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(10));
            let play = || -> () {
                let path = songs
                    .read()
                    .unwrap()
                    .get(playing.read().unwrap().unwrap())
                    .unwrap()
                    .1
                    .to_path_buf();

                player.write().unwrap().play(path);
            };

            let temp = &*songs.read().unwrap();

            if !temp.is_empty() && playing.read().unwrap().is_none() {
                //nothing is playing, but there are songs in the queue
                *playing.write().unwrap() = Some(0);
                play();
            }

            //trigger for next track
            if player.read().unwrap().next_track.load(Ordering::SeqCst) {
                if playing.read().unwrap().unwrap() != songs.read().unwrap().len() - 1 {
                    let next_track = playing.read().unwrap().unwrap();
                    *playing.write().unwrap() = Some(next_track + 1);
                    play()
                } else {
                    //loop the queue
                    *playing.write().unwrap() = Some(0);
                    play();
                }
            }
        });
    }
    pub fn up(&mut self) {
        if self.selection != 0 {
            self.selection -= 1;
        } else {
            self.selection = self.songs.read().unwrap().len() - 1;
        }
    }
    pub fn down(&mut self) {
        if self.selection != self.songs.read().unwrap().len() - 1 {
            self.selection += 1;
        } else {
            self.selection = 0;
        }
    }
}

#[derive(PartialEq)]
pub enum Mode {
    Artist,
    Album,
    Track,
}

use std::{collections::HashMap, thread};

use crate::{database::Artist, index::get_artists, list::List, player::Player};

pub struct MusicLibrary {
    music: HashMap<String, Artist>,
    mode: Mode,
    artist: List,
    album: List,
    track: List,
}
impl MusicLibrary {
    pub fn new() -> Self {
        let music = get_artists();

        let artist = List::from_vec(music.iter().map(|(_, v)| v.name.clone()).collect());
        Self {
            music,
            mode: Mode::Artist,
            artist,
            album: List::new(),
            track: List::new(),
        }
    }
    pub fn next_mode(&mut self) {
        match self.mode {
            //into album
            Mode::Artist => {
                //update renderer
                self.mode = Mode::Album;

                //update the albums
                let artist = self.artist.selected();
                self.album = List::from_vec(self.get_albums(&artist));
            }
            //track
            Mode::Album => {
                self.mode = Mode::Track;
                //update the tracks
                let artist = self.artist.selected();
                let album = self.album.selected();
                self.track = List::from_vec(self.get_tracks(&artist, &album));
            }
            //play track
            Mode::Track => {
                let artist = self.artist.selected();
                let album = self.album.selected();
                let track = self.track.selection as u16 + 1;
                self.play(&artist, &album, &track);
            }
        }
    }
    pub fn prev_mode(&mut self) {
        match self.mode {
            Mode::Artist => {}
            Mode::Album => {
                //exit to artist mode
                self.mode = Mode::Artist;

                //we want to be on album 0 next time we change modes
                self.album.clear_selection();
            }
            Mode::Track => {
                self.mode = Mode::Album;

                //we want to be on track 0 next time we change modes
                self.track.clear_selection();
            }
        }
    }
    pub fn selection(&self) -> Option<usize> {
        match self.mode {
            Mode::Artist => Some(self.artist.selection),
            Mode::Album => Some(self.album.selection),
            Mode::Track => Some(self.track.selection),
        }
    }
    pub fn items(&self) -> Vec<String> {
        match self.mode {
            Mode::Artist => self.artist.items.clone(),
            Mode::Album => self.album.items.clone(),
            Mode::Track => self.track.items.clone(),
        }
    }
    pub fn title(&self) -> String {
        match self.mode {
            Mode::Artist => String::from("Artist"),
            Mode::Album => String::from("Album"),
            Mode::Track => String::from("Track"),
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            Mode::Artist => self.artist.up(),
            Mode::Album => self.album.up(),
            Mode::Track => self.track.up(),
        }
    }
    pub fn down(&mut self) {
        match self.mode {
            Mode::Artist => self.artist.down(),
            Mode::Album => self.album.down(),
            Mode::Track => self.track.down(),
        }
    }
    fn get_albums(&self, artist: &String) -> Vec<String> {
        self.music
            .get(artist)
            .unwrap()
            .albums
            .iter()
            .map(|album| album.title.clone())
            .collect()
    }
    fn get_tracks(&self, artist: &String, album: &String) -> Vec<String> {
        self.music
            .get(artist)
            .unwrap()
            .album(&album)
            .unwrap()
            .songs
            .iter()
            .map(|song| {
                let mut out = song.number.to_string();
                out.push_str(". ");
                out.push_str(&song.title);
                out.clone()
            })
            .collect()
    }
    fn play(&mut self, artist: &String, album: &String, track: &u16) {
        let path = &self
            .music
            .get(artist)
            .unwrap()
            .album(album)
            .unwrap()
            .track(track)
            .unwrap()
            .path;

        let p = path.clone();

        thread::spawn(move || {
            Player::play(&p);
        });
    }
}

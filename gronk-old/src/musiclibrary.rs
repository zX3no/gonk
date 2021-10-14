#[derive(PartialEq)]
pub enum Mode {
    Artist,
    Album,
    Track,
}

use crossterm::style::Stylize;

use crate::{
    database::{Database, Song},
    list::List,
    player::Player,
    queue::Queue,
};

pub struct MusicLibrary {
    database: Database,
    mode: Mode,
    artist: List,
    album: List,
    track: List,
    // pub player: Player,
    queue: Queue,
}
impl MusicLibrary {
    pub fn new() -> Self {
        let database = Database::create();
        let artist = List::from_vec(MusicLibrary::get_artists(&database));

        let queue = Queue::new();
        queue.run();

        Self {
            database,
            mode: Mode::Artist,
            artist,
            album: List::new(),
            track: List::new(),
            // player: Player::new(),
            queue,
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
                self.track = List::from_vec(self.get_album(&artist, &album));
            }
            //play track
            Mode::Track => {
                let artist = self.artist.selected();
                let album = self.album.selected();

                let selected = self.track.selected();
                let num = selected.split_once('.').unwrap();
                let track = num.0.parse::<u16>().unwrap();

                // self.play(&artist, &album, &track);
            }
        }
    }
    pub fn prev_mode(&mut self) {
        match self.mode {
            Mode::Artist => {}
            Mode::Album => {
                //exit to artist mode
                self.mode = Mode::Artist;

                //update incase of search but keep previously selected
                let s = self.artist.selected();
                self.reset_filter();
                self.artist.selection = self
                    .artist
                    .items
                    .iter()
                    .position(|item| item == &s)
                    .unwrap();

                //we want to be on album 0 next time we change modes
                self.album.clear_selection();
            }
            Mode::Track => {
                self.mode = Mode::Album;

                // self.reset_filter();

                //we want to be on track 0 next time we change modes
                self.track.clear_selection();
            }
        }
    }
    pub fn browser_selection(&self) -> Option<usize> {
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
    pub fn filter(&mut self, query: &str) {
        match self.mode {
            Mode::Artist => self.artist.filter(query),
            Mode::Album => self.album.filter(query),
            Mode::Track => self.track.filter(query),
        };
    }
    pub fn reset_filter(&mut self) {
        match self.mode {
            Mode::Artist => self.artist = List::from_vec(MusicLibrary::get_artists(&self.database)),
            Mode::Album => self.album = List::from_vec(self.get_albums(&self.artist.selected())),
            Mode::Track => {
                self.track =
                    List::from_vec(self.get_album(&self.artist.selected(), &self.album.selected()))
            }
        };
    }
    pub fn filter_len(&mut self) -> usize {
        match self.mode {
            Mode::Artist => self.artist.len(),
            Mode::Album => self.album.len(),
            Mode::Track => self.track.len(),
        }
    }
    pub fn add_to_queue(&mut self) {
        match self.mode {
            Mode::Artist => (),
            Mode::Album => (),
            Mode::Track => {
                let artist = self.artist.selected();
                let album = self.album.selected();

                let selected = self.track.selected();
                let num = selected.split_once('.').unwrap();
                let track = num.0.parse::<u16>().unwrap();

                let song = &self
                    .database
                    .get_song(&artist.to_string(), &album.to_string(), &track);

                self.queue.add(song.name.clone(), &song.path);

                // self.play(&artist, &album, &track);
            }
        }
    }
    fn get_artists(database: &Database) -> Vec<String> {
        let mut a: Vec<String> = database
            .get_artists()
            .iter()
            .map(|a| a.name.clone())
            .collect();
        // a.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        a.sort_by_key(|a| a.to_lowercase());
        return a;
    }
    fn get_albums(&self, artist: &str) -> Vec<String> {
        self.database
            .get_albums_by_artist(&artist.to_string())
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }
    fn get_album(&self, artist: &str, album: &str) -> Vec<String> {
        let mut album: Vec<Song> = self
            .database
            .get_album(&artist.to_string(), &album.to_string());

        album.sort_by(|a, b| {
            a.disc
                .cmp(&b.disc)
                .then(a.track_number.cmp(&b.track_number))
        });

        album.iter().map(|s| s.name_with_number.clone()).collect()
    }
    pub fn toggle_playback(&self) {
        self.queue.player.write().unwrap().toggle_playback();
    }
    pub fn decrease_volume(&self) {
        self.queue.player.write().unwrap().decrease_volume();
    }
    pub fn increase_volume(&self) {
        self.queue.player.write().unwrap().increase_volume();
    }
    pub fn volume(&self) -> String {
        self.queue.player.write().unwrap().volume.to_string()
    }
    pub fn now_playing(&self) -> String {
        self.queue.player.write().unwrap().now_playing()
    }
    pub fn progress(&self) -> String {
        self.queue.player.write().unwrap().progress()
    }
    pub fn progress_percent(&self) -> u16 {
        self.queue.player.write().unwrap().progress_percent()
    }
    pub fn queue(&self) -> Vec<String> {
        self.queue
            .songs
            .read()
            .unwrap()
            .iter()
            .map(|(s, _)| s.clone())
            .collect()
    }
    pub fn queue_selection(&self) -> Option<usize> {
        Some(self.queue.selection)
    }
    pub fn queue_up(&mut self) {
        self.queue.up();
    }
    pub fn queue_down(&mut self) {
        self.queue.down();
    }
    pub fn next(&mut self) {
        self.queue.next();
    }
    // fn play(&mut self, artist: &str, album: &str, track: &u16) {
    //     self.player.play(
    //         &self
    //             .database
    //             .get_song(&artist.to_string(), &album.to_string(), track)
    //             .path,
    //     );
    // }
}

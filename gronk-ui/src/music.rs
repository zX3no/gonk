use gronk_database::database::Database;
use gronk_player::{player::Player, queue::Queue};

use crate::app::BrowserMode;
pub struct Item {
    //optional track number, this can be done better
    pub prefix: Option<u16>,
    pub item: String,
    pub index: usize,
    pub len: usize,
}
impl Item {
    pub fn new(prefix: Option<u16>, item: String, index: usize, len: usize) -> Self {
        Self {
            prefix,
            item,
            index,
            len,
        }
    }
}

pub struct Music {
    database: Database,
    selected_artist: Item,
    selected_album: Item,
    selected_song: Item,
    artists: Vec<String>,
    albums: Vec<String>,
    songs: Vec<(u16, String)>,
    player: Player,
    //TODO: serialize volume so
    //it's the same when you reopen
    //volume: f64,
}

impl Music {
    pub fn new() -> Self {
        let database = Database::new();

        let artists = database.get_artists().unwrap();
        let artist = artists.first().unwrap().clone();

        let albums = database.get_albums_by_artist(&artist).unwrap();
        let album = albums.first().unwrap().clone();

        let songs = database.get_songs_from_album(&artist, &album).unwrap();
        let (num, name) = songs.first().unwrap().clone();

        Self {
            database,
            selected_artist: Item::new(None, artist, 0, artists.len()),
            selected_album: Item::new(None, album, 0, albums.len()),
            selected_song: Item::new(Some(num), name, 0, songs.len()),
            artists,
            albums,
            songs,
            player: Player::new(),
        }
    }
    pub fn update_db(&self) {
        todo!();
    }
    pub fn volume_up(&self) {
        self.player.volume(0.005);
    }
    pub fn volume_down(&self) {
        self.player.volume(-0.005);
    }
    pub fn play_pause(&self) {
        self.player.toggle_playback();
    }
    pub fn next(&self) {
        self.player.next();
    }
    pub fn prev(&self) {
        self.player.previous();
    }
    pub fn clear(&self) {
        self.player.clear_queue();
    }
    pub fn queue_artist(&self) {
        let songs = self.database.get_artist(&self.selected_artist.item);
        self.player.add(songs);
    }
    pub fn queue_album(&self) {
        let songs = self
            .database
            .get_album(&self.selected_artist.item, &self.selected_album.item);
        self.player.add(songs);
    }
    pub fn queue_song(&self) {
        let song = self.database.get_song(
            &self.selected_artist.item,
            &self.selected_album.item,
            &self.selected_song.prefix.unwrap(),
            &self.selected_song.item,
        );
        self.player.add(song);
    }
    pub fn get_selected_artist(&self) -> Option<usize> {
        Some(self.selected_artist.index)
    }
    pub fn get_selected_album(&self) -> Option<usize> {
        Some(self.selected_album.index)
    }
    pub fn get_selected_song(&self) -> Option<usize> {
        Some(self.selected_song.index)
    }
    pub fn artist_names(&self) -> &Vec<String> {
        &self.artists
    }
    pub fn album_names(&self) -> &Vec<String> {
        &self.albums
    }
    pub fn song_names(&self) -> Vec<String> {
        self.songs
            .iter()
            .map(|song| format!("{}. {}", song.0, song.1))
            .collect()
    }
    pub fn up(&mut self, mode: &BrowserMode) {
        let item = match mode {
            BrowserMode::Artist => &mut self.selected_artist,
            BrowserMode::Album => &mut self.selected_album,
            BrowserMode::Song => &mut self.selected_song,
        };

        if item.index > 0 {
            item.index -= 1;
        } else {
            item.index = item.len - 1;
        }

        if let BrowserMode::Artist = mode {
            self.reset_artist();
        } else if let BrowserMode::Album = mode {
            self.update_song();
        }
    }
    pub fn down(&mut self, mode: &BrowserMode) {
        let item = match mode {
            BrowserMode::Artist => &mut self.selected_artist,
            BrowserMode::Album => &mut self.selected_album,
            BrowserMode::Song => &mut self.selected_song,
        };

        if item.index + 1 < item.len {
            item.index += 1;
        } else {
            item.index = 0;
        }

        match mode {
            BrowserMode::Artist => self.reset_artist(),
            BrowserMode::Album => self.reset_songs(),
            BrowserMode::Song => self.update_song(),
        }
    }
    pub fn update_song(&mut self) {
        let (number, name) = self.songs.get(self.selected_song.index).unwrap().clone();
        self.selected_song.prefix = Some(number);
        self.selected_song.item = name;
    }
    pub fn reset_artist(&mut self) {
        //Update the album based on artist selection
        self.selected_artist.item = self
            .artists
            .get(self.selected_artist.index)
            .unwrap()
            .to_owned();

        self.albums = self
            .database
            .get_albums_by_artist(&self.selected_artist.item)
            .unwrap();

        self.selected_album = Item::new(
            None,
            self.albums.first().unwrap().clone(),
            0,
            self.albums.len(),
        );

        self.reset_songs();
    }
    pub fn reset_songs(&mut self) {
        //Update the song based on album selection
        self.selected_album.item = self
            .albums
            .get(self.selected_album.index)
            .unwrap()
            .to_owned();

        self.songs = self
            .database
            .get_songs_from_album(&self.selected_artist.item, &self.selected_album.item)
            .unwrap();

        let (num, name) = self.songs.first().unwrap().clone();
        self.selected_song = Item::new(Some(num), name, 0, self.songs.len());
    }

    pub fn get_queue(&self) -> Queue {
        self.player.get_queue()
    }
}

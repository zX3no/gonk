use crate::modes::BrowserMode;
use gronk_database::Database;

//TODO: wtf is this type??? change to templates
#[derive(Debug)]
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

pub struct Browser<'a> {
    db: &'a Database,
    pub selected_artist: Item,
    pub selected_album: Item,
    pub selected_song: Item,
    mode: BrowserMode,
    artists: Vec<String>,
    albums: Vec<String>,
    songs: Vec<(u16, String)>,
    //TODO: serialize volume so
    //it's the same when you reopen
    //volume: f64,
    //maybe do this on queue?
}

impl<'a> Browser<'a> {
    pub fn new(db: &'a Database) -> Self {
        let artists = db.artists().unwrap();
        let artist = artists.first().unwrap().clone();

        let albums = db.albums_by_artist(&artist).unwrap();
        let album = albums.first().unwrap().clone();

        let songs = db.songs_from_album(&artist, &album).unwrap();
        let (num, name) = songs.first().unwrap().clone();

        Self {
            db,
            selected_artist: Item::new(None, artist, 0, artists.len()),
            selected_album: Item::new(None, album, 0, albums.len()),
            selected_song: Item::new(Some(num), name, 0, songs.len()),
            mode: BrowserMode::Artist,
            artists,
            albums,
            songs,
        }
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
    pub fn update_browser(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.reset_artist(),
            BrowserMode::Album => self.reset_songs(),
            BrowserMode::Song => self.update_song(),
        }
    }
    pub fn get_item(&mut self) -> &mut Item {
        match self.mode {
            BrowserMode::Artist => &mut self.selected_artist,
            BrowserMode::Album => &mut self.selected_album,
            BrowserMode::Song => &mut self.selected_song,
        }
    }
    pub fn up(&mut self) {
        let item = self.get_item();

        if item.index > 0 {
            item.index -= 1;
        } else {
            item.index = item.len - 1;
        }
        self.update_browser();
    }
    pub fn down(&mut self) {
        let item = self.get_item();

        if item.index + 1 < item.len {
            item.index += 1;
        } else {
            item.index = 0;
        }
        self.update_browser();
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
            .db
            .albums_by_artist(&self.selected_artist.item)
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
            .db
            .songs_from_album(&self.selected_artist.item, &self.selected_album.item)
            .unwrap();

        let (num, name) = self.songs.first().unwrap().clone();
        self.selected_song = Item::new(Some(num), name, 0, self.songs.len());
    }
    pub fn next(&mut self) {
        self.mode.next();
    }
    pub fn prev(&mut self) {
        self.mode.prev();
    }
    pub fn mode(&self) -> &BrowserMode {
        &self.mode
    }
}

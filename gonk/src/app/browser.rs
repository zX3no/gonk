use crate::index::Index;
use gonk_database::Database;
use gonk_types::Song;

pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

impl BrowserMode {
    pub fn next(&mut self) {
        match self {
            BrowserMode::Artist => *self = BrowserMode::Album,
            BrowserMode::Album => *self = BrowserMode::Song,
            BrowserMode::Song => (),
        }
    }
    pub fn prev(&mut self) {
        match self {
            BrowserMode::Artist => (),
            BrowserMode::Album => *self = BrowserMode::Artist,
            BrowserMode::Song => *self = BrowserMode::Album,
        }
    }
}

pub struct Browser<'a> {
    db: &'a Database,
    artists: Index<String>,
    albums: Index<String>,
    songs: Index<(u16, String)>,
    pub mode: BrowserMode,
    is_busy: bool,
}

impl<'a> Browser<'a> {
    pub fn new(db: &'a Database) -> Self {
        let artists = Index::new(db.artists(), Some(0));

        let (albums, songs) = if let Some(first_artist) = artists.selected() {
            let albums = Index::new(db.albums_by_artist(first_artist), Some(0));

            if let Some(first_album) = albums.selected() {
                let songs = db.songs_from_album(first_artist, first_album);
                (albums, Index::new(songs, Some(0)))
            } else {
                (albums, Index::default())
            }
        } else {
            (Index::default(), Index::default())
        };

        Self {
            db,
            artists,
            albums,
            songs,
            mode: BrowserMode::Artist,
            is_busy: false,
        }
    }
    pub fn update_busy(&mut self, busy: bool) {
        self.is_busy = busy;
    }
    pub fn is_busy(&self) -> bool {
        self.is_busy
    }
    pub fn get_selected_artist(&self) -> Option<usize> {
        self.artists.index
    }
    pub fn get_selected_album(&self) -> Option<usize> {
        self.albums.index
    }
    pub fn get_selected_song(&self) -> Option<usize> {
        self.songs.index
    }
    pub fn artist_names(&self) -> &Vec<String> {
        &self.artists.data
    }
    pub fn album_names(&self) -> &Vec<String> {
        &self.albums.data
    }
    pub fn song_names(&self) -> Vec<String> {
        self.songs
            .data
            .iter()
            .map(|song| format!("{}. {}", song.0, song.1))
            .collect()
    }
    pub fn up(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.artists.up(),
            BrowserMode::Album => self.albums.up(),
            BrowserMode::Song => self.songs.up(),
        }
        self.update_browser();
    }
    pub fn down(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.artists.down(),
            BrowserMode::Album => self.albums.down(),
            BrowserMode::Song => self.songs.down(),
        }
        self.update_browser();
    }
    pub fn update_browser(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.update_albums(),
            BrowserMode::Album => self.update_songs(),
            BrowserMode::Song => (),
        }
    }
    pub fn update_albums(&mut self) {
        //Update the album based on artist selection
        if let Some(name) = self.artists.selected() {
            self.albums.data = self.db.albums_by_artist(name);
            self.albums.select(Some(0));

            self.update_songs();
        }
    }
    pub fn update_songs(&mut self) {
        if let Some(artist) = self.artists.selected() {
            if let Some(album) = self.albums.selected() {
                self.songs.data = self.db.songs_from_album(artist, album);
                self.songs.select(Some(0));
            }
        }
    }
    pub fn next(&mut self) {
        self.mode.next();
    }
    pub fn prev(&mut self) {
        self.mode.prev();
    }
    pub fn on_enter(&self) -> Vec<Song> {
        let artist = self.artists.selected().unwrap();
        let album = self.albums.selected().unwrap();
        let song = self.songs.selected().unwrap();
        match self.mode {
            BrowserMode::Artist => self.db.get_artist(artist),
            BrowserMode::Album => self.db.get_album(artist, album),
            BrowserMode::Song => self.db.get_song(artist, album, song),
        }
    }
    pub fn refresh(&mut self) {
        if self.artists.is_empty() {
            self.artists = Index::new(self.db.artists(), Some(0));
        } else {
            self.artists.data = self.db.artists();
        }

        if let Some(first_artist) = self.artists.selected() {
            if self.albums.is_empty() {
                self.albums = Index::new(self.db.albums_by_artist(first_artist), Some(0));
            } else {
                self.albums.data = self.db.albums_by_artist(first_artist);
            }
        }

        if let Some(first_artist) = self.artists.selected() {
            if let Some(first_album) = self.albums.selected() {
                if self.songs.is_empty() {
                    self.songs =
                        Index::new(self.db.songs_from_album(first_artist, first_album), Some(0));
                } else {
                    self.songs.data = self.db.songs_from_album(first_artist, first_album);
                }
            }
        }
    }
    pub fn reset(&mut self) {
        self.artists = Index::default();
        self.albums = Index::default();
        self.songs = Index::default();
        self.mode = BrowserMode::Artist;
    }
}

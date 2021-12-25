use crate::{index::Index, modes::BrowserMode};
use gronk_database::Database;

struct Song(u16, String);

static EMPTY: Vec<String> = Vec::new();

#[derive(Debug)]
struct Item<T> {
    item: T,
    index: Index,
}
impl Item<Song> {
    pub fn new(number: u16, name: String) -> Self {
        Item {
            item: Song(number, name),
            index: Index::new(Some(0)),
        }
    }
}
impl Item<String> {
    pub fn new(name: String) -> Self {
        Item {
            item: name,
            index: Index::new(Some(0)),
        }
    }
}
pub struct BrowserData {
    selected_artist: Item<String>,
    selected_album: Item<String>,
    selected_song: Item<Song>,
    artists: Vec<String>,
    albums: Vec<String>,
    songs: Vec<(u16, String)>,
}
impl BrowserData {
    pub fn songs(&self) -> Vec<String> {
        self.songs
            .iter()
            .map(|song| format!("{}. {}", song.0, song.1))
            .collect()
    }
    pub fn song(&self) -> (u16, &str) {
        (self.selected_song.item.0, &self.selected_song.item.1)
    }
    pub fn album(&self) -> &String {
        &self.selected_album.item
    }
    pub fn artist(&self) -> &String {
        &self.selected_artist.item
    }
}

pub struct Browser<'a> {
    db: &'a Database,
    pub browser_data: Option<BrowserData>,
    mode: BrowserMode,
}

impl<'a> Browser<'a> {
    pub fn new(db: &'a Database) -> Self {
        let artists = db.artists();
        if let Some(artist) = artists.first() {
            let selected_artist = Item::<String>::new(artist.clone());

            let albums = db.albums_by_artist(&artist);

            if let Some(album) = albums.first() {
                let selected_album = Item::<String>::new(album.clone());

                let songs = db.songs_from_album(&artist, &album);

                if let Some((number, name)) = songs.first() {
                    let selected_song = Item::<Song>::new(*number, name.clone());

                    return Self {
                        db,
                        browser_data: Some(BrowserData {
                            selected_artist,
                            selected_album,
                            selected_song,
                            artists,
                            albums,
                            songs,
                        }),
                        mode: BrowserMode::Artist,
                    };
                };
            }
        };

        Self {
            db,
            mode: BrowserMode::Artist,
            browser_data: None,
        }
    }
    pub fn get_selected_artist(&self) -> Option<usize> {
        if let Some(data) = &self.browser_data {
            data.selected_artist.index.selected()
        } else {
            None
        }
    }
    pub fn get_selected_album(&self) -> Option<usize> {
        if let Some(data) = &self.browser_data {
            data.selected_album.index.selected()
        } else {
            None
        }
    }
    pub fn get_selected_song(&self) -> Option<usize> {
        if let Some(data) = &self.browser_data {
            data.selected_song.index.selected()
        } else {
            None
        }
    }
    pub fn artist_names(&self) -> &Vec<String> {
        if let Some(data) = &self.browser_data {
            &data.artists
        } else {
            &EMPTY
        }
    }
    pub fn album_names(&self) -> &Vec<String> {
        if let Some(data) = &self.browser_data {
            &data.albums
        } else {
            &EMPTY
        }
    }
    pub fn song_names(&self) -> Vec<String> {
        if let Some(data) = &self.browser_data {
            data.songs()
        } else {
            Vec::new()
        }
    }
    pub fn up(&mut self) {
        if let Some(data) = &mut self.browser_data {
            match self.mode {
                BrowserMode::Artist => {
                    data.selected_artist.index.up(data.artists.len());
                }
                BrowserMode::Album => {
                    data.selected_album.index.up(data.albums.len());
                }
                BrowserMode::Song => {
                    data.selected_song.index.up(data.songs.len());
                }
            }
        }
        self.update_browser();
    }
    pub fn down(&mut self) {
        if let Some(data) = &mut self.browser_data {
            match self.mode {
                BrowserMode::Artist => {
                    data.selected_artist.index.down(data.artists.len());
                }
                BrowserMode::Album => {
                    data.selected_album.index.down(data.albums.len());
                }
                BrowserMode::Song => {
                    data.selected_song.index.down(data.songs.len());
                }
            }
        }
        self.update_browser();
    }
    pub fn update_browser(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.update_albums(),
            BrowserMode::Album => self.update_songs(),
            BrowserMode::Song => self.update_selected_song(),
        }
    }
    pub fn update_albums(&mut self) {
        //Update the album based on artist selection
        if let Some(data) = &mut self.browser_data {
            let artist = &mut data.selected_artist;
            if let Some(index) = artist.index.selected() {
                let name = data.artists.get(index).unwrap();
                artist.item = name.clone();

                data.albums = self.db.albums_by_artist(&name);

                data.selected_album = Item::<String>::new(data.albums.first().unwrap().to_string());

                self.update_songs();
            }
        }
    }
    pub fn update_songs(&mut self) {
        //Update the song based on album selection
        if let Some(data) = &mut self.browser_data {
            let album = &mut data.selected_album;
            let index = album.index.selected().unwrap_or(0);
            let name = data.albums.get(index).unwrap();
            album.item = name.clone();
            data.songs = self.db.songs_from_album(&data.selected_artist.item, name);

            let (number, name) = data.songs.first().unwrap();
            data.selected_song = Item::<Song>::new(*number, name.clone());
        }
    }
    pub fn update_selected_song(&mut self) {
        if let Some(data) = &mut self.browser_data {
            let (sel, songs) = (&mut data.selected_song, &data.songs);
            let index = sel.index.selected().unwrap_or(0);
            if let Some((number, name)) = songs.get(index) {
                sel.item = Song(*number, name.clone());
            }
        }
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

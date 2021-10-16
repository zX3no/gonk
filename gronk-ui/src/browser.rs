use gronk_indexer::database::Database;
use tui::widgets::ListItem;

pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

pub struct Browser<'a> {
    pub mode: BrowserMode,
    pub artist: BrowserList<'a>,
    pub album: BrowserList<'a>,
    pub song: BrowserList<'a>,
    pub database: Database,
}

impl<'a> Browser<'a> {
    pub fn new() -> Self {
        let database = Database::new();
        let artist = BrowserList::artist(&database);
        let album = BrowserList::album(&database);
        let song = BrowserList::song(&database);

        Self {
            mode: BrowserMode::Artist,
            artist,
            album,
            song,
            database,
        }
    }
    pub fn get(&self) -> &Vec<ListItem<'a>> {
        return match self.mode {
            BrowserMode::Artist => &self.artist.list,
            BrowserMode::Album => &self.album.list,
            BrowserMode::Song => &self.song.list,
        };
    }
    pub fn filter_album_by_artist() {
        todo!();
    }
    pub fn filter_song_by_album() {
        todo!();
    }
}

pub struct BrowserList<'a> {
    list: Vec<ListItem<'a>>,
    selection: usize,
}

impl<'a> BrowserList<'a> {
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            selection: 0,
        }
    }
    pub fn artist(database: &Database) -> Self {
        let mut artists = Vec::new();
        for artist in &database.artists {
            artists.push(&artist.name);
        }

        Self {
            list: BrowserList::from_strings(artists),
            selection: 0,
        }
    }
    pub fn album(database: &Database) -> Self {
        let mut albums = Vec::new();
        for album in &database.get_albums() {
            albums.push(&album.name);
        }

        Self {
            list: BrowserList::from_strings(albums),
            selection: 0,
        }
    }
    pub fn song(database: &Database) -> Self {
        let mut songs = Vec::new();
        for song in &database.get_albums() {
            songs.push(&song.name);
        }

        Self {
            list: BrowserList::from_strings(songs),
            selection: 0,
        }
    }
    pub fn from_strings(strings: Vec<&String>) -> Vec<ListItem<'a>> {
        let mut list = Vec::new();
        for item in strings {
            list.push(ListItem::new(item.clone()));
        }
        return list;
    }
}

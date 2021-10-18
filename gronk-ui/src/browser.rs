use gronk_indexer::database::Database;
use tui::widgets::{ListItem, ListState};

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
    fn get_selection(&mut self) -> &mut BrowserList<'a> {
        return match self.mode {
            BrowserMode::Artist => &mut self.artist,
            BrowserMode::Album => &mut self.album,
            BrowserMode::Song => &mut self.song,
        };
    }
    pub fn selected(&mut self) -> &mut ListState {
        return match self.mode {
            BrowserMode::Artist => &mut self.artist.selection,
            BrowserMode::Album => &mut self.album.selection,
            BrowserMode::Song => &mut self.song.selection,
        };
    }
    pub fn next_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.mode = BrowserMode::Album,
            BrowserMode::Album => self.mode = BrowserMode::Song,
            BrowserMode::Song => (),
        }
    }
    pub fn prev_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => (),
            BrowserMode::Album => self.mode = BrowserMode::Artist,
            BrowserMode::Song => self.mode = BrowserMode::Album,
        }
    }
    pub fn title(&self) -> String {
        return match self.mode {
            BrowserMode::Artist => String::from("Artist"),
            BrowserMode::Album => String::from("Album"),
            BrowserMode::Song => String::from("Song"),
        };
    }
    pub fn up(&mut self) {
        let selection = self.get_selection();
        selection.up();
    }
    pub fn down(&mut self) {
        let selection = self.get_selection();
        selection.down();
    }
    pub fn is_song(&self) -> bool {
        if let BrowserMode::Song = self.mode {
            return true;
        }
        return false;
    }
    pub fn filter_album_by_artist() {
        todo!();
    }
    pub fn filter_song_by_album() {
        todo!();
    }
}

//change browser list to three different types
//artist
//album
//song
//they all derive the trait Browser
//the trait is
//up
//down
pub struct BrowserList<'a> {
    list: Vec<ListItem<'a>>,
    selection: ListState,
}

impl<'a> BrowserList<'a> {
    pub fn down(&mut self) {
        let len = self.list.len();
        let selection = &mut self.selection;
        let selected = selection.selected();

        if let Some(selected) = selected {
            if selected + 1 > len - 1 {
                selection.select(Some(0));
            } else {
                selection.select(Some(selected + 1));
            }
        }
    }
    pub fn up(&mut self) {
        let len = self.list.len();
        let selection = &mut self.selection;
        let selected = selection.selected();

        if let Some(selected) = selected {
            if selected != 0 {
                selection.select(Some(selected - 1));
            } else {
                selection.select(Some(len - 1));
            }
        }
    }
    pub fn artist(database: &Database) -> Self {
        let mut artists = Vec::new();
        for artist in &database.artists {
            artists.push(&artist.name);
        }

        let mut selection = ListState::default();
        selection.select(Some(0));
        Self {
            list: BrowserList::from_strings(artists),
            selection,
        }
    }
    pub fn album(database: &Database) -> Self {
        let mut albums = Vec::new();
        for album in &database.get_albums() {
            albums.push(&album.name);
        }

        let mut selection = ListState::default();
        selection.select(Some(0));

        Self {
            list: BrowserList::from_strings(albums),
            selection,
        }
    }
    pub fn song(database: &Database) -> Self {
        let mut songs = Vec::new();
        for song in &database.get_songs() {
            songs.push(&song.name);
        }

        let mut selection = ListState::default();
        selection.select(Some(0));
        Self {
            list: BrowserList::from_strings(songs),
            selection,
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

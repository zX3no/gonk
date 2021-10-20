use gronk_indexer::database::{Album, Artist, Database, Song};
use tui::widgets::{ListItem, ListState};

#[derive(Debug, Clone)]
pub enum BrowserMode {
    Artist,
    Album,
    Song,
}

#[derive(Debug, Clone)]
pub struct Browser {
    pub mode: BrowserMode,
    pub artist: List<Artist>,
    pub album: List<Album>,
    pub song: List<Song>,
    pub database: Database,
}

impl Browser {
    pub fn new() -> Self {
        let database = Database::new(r"D:\OneDrive\Music");
        let artist = List::<Artist>::new(&database);
        let album = List::<Album>::new(&database, artist.first());
        let song = List::<Song>::new(&database, album.first());

        Self {
            mode: BrowserMode::Artist,
            artist,
            album,
            song,
            database,
        }
    }
    //updates the albums or songs depending on what was selected
    pub fn update(&mut self) {
        match self.mode {
            BrowserMode::Album => {
                self.album
                    .update(&self.database, &self.artist.get_selected());
            }
            BrowserMode::Song => {
                self.song.update(&self.database, &self.album.get_selected());
            }
            _ => (),
        }
    }
    pub fn get_list_items(&self) -> Vec<ListItem<'static>> {
        return match self.mode {
            BrowserMode::Artist => self.artist.get_list(),
            BrowserMode::Album => self.album.get_list(),
            BrowserMode::Song => self.song.get_list(),
        };
    }

    pub fn get_selection(&mut self) -> &mut ListState {
        return match self.mode {
            BrowserMode::Artist => &mut self.artist.state,
            BrowserMode::Album => &mut self.album.state,
            BrowserMode::Song => &mut self.song.state,
        };
    }
    pub fn next_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.mode = BrowserMode::Album,
            BrowserMode::Album => self.mode = BrowserMode::Song,
            BrowserMode::Song => (),
        }
        self.update();
    }
    pub fn get_song(&self) -> Option<&Song> {
        if let BrowserMode::Song = self.mode {
            let song = self.song.get_selected_data().unwrap();
            let song = self.database.find_song(&song.name);
            return song;
        }
        return None;
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
        match self.mode {
            BrowserMode::Artist => self.artist.up(),
            BrowserMode::Album => self.album.up(),
            BrowserMode::Song => self.song.up(),
        };
    }
    pub fn down(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.artist.down(),
            BrowserMode::Album => self.album.down(),
            BrowserMode::Song => self.song.down(),
        };
    }
    pub fn is_song(&self) -> bool {
        if let BrowserMode::Song = self.mode {
            return true;
        }
        return false;
    }
}

#[derive(Debug, Clone)]
pub struct List<T> {
    list: Vec<(String, T)>,
    state: ListState,
}

impl<T: Clone> List<T> {
    fn up(&mut self) {
        let len = self.list.len();
        if let Some(selected) = self.state.selected() {
            if selected != 0 {
                self.state.select(Some(selected - 1));
            } else {
                self.state.select(Some(len - 1));
            }
        }
    }

    fn down(&mut self) {
        let len = self.list.len();
        if let Some(selected) = self.state.selected() {
            if selected + 1 > len - 1 {
                self.state.select(Some(0));
            } else {
                self.state.select(Some(selected + 1));
            }
        }
    }

    fn get_selected_data(&self) -> Option<T> {
        if let Some(index) = self.state.selected() {
            return Some(self.list.get(index).unwrap().1.clone());
        }
        None
    }

    fn get_selected(&self) -> String {
        if let Some(index) = self.state.selected() {
            return self.list.get(index).unwrap().0.clone();
        }
        String::new()
    }

    fn get_list(&self) -> Vec<ListItem<'static>> {
        self.list
            .iter()
            .map(|(item, _)| ListItem::new(item.clone()))
            .collect()
    }

    fn first(&self) -> &String {
        &self.list.first().unwrap().0
    }
}
impl List<Artist> {
    pub fn new(database: &Database) -> Self {
        let mut list = Vec::new();
        for artist in &database.artists {
            list.push((artist.name.clone(), artist.clone()));
        }

        list.sort_by_key(|(name, _)| name.to_lowercase());

        let mut state = ListState::default();
        state.select(Some(0));

        Self { list, state }
    }
}
impl List<Album> {
    pub fn new(database: &Database, artist: &String) -> Self {
        let artist = database.find_artist(artist).unwrap();

        let mut list = Vec::new();
        for album in &artist.albums {
            list.push((album.name.clone(), album.clone()));
        }

        list.sort_by_key(|(name, _)| name.to_lowercase());

        let mut state = ListState::default();
        state.select(Some(0));

        Self { list, state }
    }
    pub fn update(&mut self, database: &Database, name: &String) {
        let artist = database.find_artist(&name).unwrap();
        let mut list: Vec<(String, Album)> = artist
            .albums
            .iter()
            .map(|album| (album.name.clone(), album.clone()))
            .collect();

        list.sort_by_key(|(name, _)| name.to_lowercase());

        self.state.select(Some(0));
        self.list = list;
    }
}
impl List<Song> {
    pub fn new(database: &Database, album: &String) -> Self {
        let album = database.find_album(album).unwrap();

        let mut list = Vec::new();
        let mut songs = album.songs.clone();

        songs.sort_by(|a, b| {
            a.disc
                .cmp(&b.disc)
                .then(a.track_number.cmp(&b.track_number))
        });

        for songs in &songs {
            list.push((songs.name.clone(), songs.clone()));
        }

        let mut state = ListState::default();
        state.select(Some(0));

        Self { list, state }
    }
    pub fn update(&mut self, database: &Database, name: &String) {
        let album = database.find_album(&name).unwrap();

        let mut songs = album.songs.clone();

        songs.sort_by(|a, b| {
            a.disc
                .cmp(&b.disc)
                .then(a.track_number.cmp(&b.track_number))
        });

        let list = songs
            .iter()
            .map(|song| (song.name_with_number.clone(), song.clone()))
            .collect();

        self.state.select(Some(0));
        self.list = list;
    }
}

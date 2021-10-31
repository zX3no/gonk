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
    pub fn refresh(&mut self) {
        self.artist.refresh(&self.database);
        // BrowserMode::Album => self.album = List::<Album>::new(&self.database, &self.artist),
        // BrowserMode::Song => self.song = List::<Song>::new(&self.database, &self.album),
    }
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
    pub fn search(&mut self, query: &String) {
        match self.mode {
            BrowserMode::Artist => self.artist.filter(query),
            BrowserMode::Album => self.album.filter(query),
            BrowserMode::Song => self.song.filter(query),
        };
    }
    pub fn get_list_items(&self) -> Vec<ListItem<'static>> {
        match self.mode {
            BrowserMode::Artist => self.artist.get_list(),
            BrowserMode::Album => self.album.get_list(),
            BrowserMode::Song => self.song.get_list(),
        }
    }

    pub fn get_selection(&mut self) -> &mut ListState {
        match self.mode {
            BrowserMode::Artist => &mut self.artist.state,
            BrowserMode::Album => &mut self.album.state,
            BrowserMode::Song => &mut self.song.state,
        }
    }
    pub fn next_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => self.mode = BrowserMode::Album,
            BrowserMode::Album => self.mode = BrowserMode::Song,
            BrowserMode::Song => return,
        }
        self.update();
    }
    pub fn get_songs(&self) -> Vec<Song> {
        match self.mode {
            BrowserMode::Artist => {
                let artist = &self.artist.get_selected_data().unwrap();
                let mut songs = Vec::new();
                for album in &artist.albums {
                    songs.extend(album.songs.clone());
                }
                Browser::sort(songs)
            }
            BrowserMode::Album => Browser::sort(self.album.get_selected_data().unwrap().songs),
            BrowserMode::Song => vec![self.song.get_selected_data().unwrap()],
        }
    }
    fn sort(mut songs: Vec<Song>) -> Vec<Song> {
        songs.sort_by(|a, b| {
            a.album.cmp(&b.album).then(
                a.disc
                    .cmp(&b.disc)
                    .then(a.track_number.cmp(&b.track_number)),
            )
        });
        songs
    }
    pub fn prev_mode(&mut self) {
        match self.mode {
            BrowserMode::Artist => (),
            BrowserMode::Album => self.mode = BrowserMode::Artist,
            BrowserMode::Song => self.mode = BrowserMode::Album,
        }
    }
    pub fn title(&self) -> String {
        match self.mode {
            BrowserMode::Artist => String::from("Artist"),
            BrowserMode::Album => String::from("Album"),
            BrowserMode::Song => String::from("Song"),
        }
    }
    // pub fn get(&mut self) -> &mut List<T> {
    //     match self.mode {
    //         BrowserMode::Artist => &mut self.artist,
    //         BrowserMode::Album => self.album,
    //         BrowserMode::Song => self.song,
    //     }
    // }
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
        matches!(self.mode, BrowserMode::Song)
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

    pub fn filter(&mut self, query: &String) {
        self.list = self
            .list
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(query))
            .cloned()
            .collect();

        if self.list.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
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
    pub fn refresh(&mut self, database: &Database) {
        if let Some(index) = self.state.selected() {
            let name = self.list.get(index).unwrap().0.clone();
            *self = Self::new(database);

            for (i, item) in self.list.iter().enumerate() {
                if item.0 == name {
                    self.state.select(Some(i));
                }
            }
        } else {
            *self = Self::new(database);
        }
    }
}
impl List<Album> {
    pub fn new(database: &Database, artist: &str) -> Self {
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
    pub fn update(&mut self, database: &Database, name: &str) {
        let artist = database.find_artist(name).unwrap();
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
    pub fn new(database: &Database, album: &str) -> Self {
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
    pub fn update(&mut self, database: &Database, name: &str) {
        let album = database.find_album(name).unwrap();

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

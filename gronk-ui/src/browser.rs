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
    pub artist: ArtistList,
    pub album: AlbumList,
    pub song: SongList,
    pub database: Database,
}

impl Browser {
    pub fn new() -> Self {
        let database = Database::new(r"D:\OneDrive\Music");
        let artist = ArtistList::new(&database);
        let album = AlbumList::new(&database, artist.first());
        let song = SongList::new(&database, album.first());

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
        let database = self.database.clone();
        match self.mode {
            BrowserMode::Album => {
                // self.album.update(&database, self.artist.get_selected());
            }
            BrowserMode::Song => {
                // self.song =
                //     BrowserList::get_songs_from_album(&self.database, self.album.get_selected());
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

fn test(al: &ArtistList) -> Vec<ListItem<'static>> {
    al.list
        .iter()
        .map(|(item, _)| ListItem::new(item.clone()))
        .collect()
}

#[derive(Debug, Clone)]
pub struct ArtistList {
    list: Vec<(String, Artist)>,
    state: ListState,
}
impl ArtistList {
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
    pub fn test(&self) -> Vec<ListItem> {
        self.list
            .iter()
            .map(|(item, _)| ListItem::new(item.clone()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct AlbumList {
    list: Vec<(String, Album)>,
    state: ListState,
}
impl AlbumList {
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
#[derive(Debug, Clone)]
pub struct SongList {
    list: Vec<(String, Song)>,
    state: ListState,
}
impl SongList {
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
}
macro_rules! impl_foo {
    ($($t:ty),+) => {
        $(impl List for $t {

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

    fn get_selected(&self) -> String {
        if let Some(index) = self.state.selected(){
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
        })+
    }


}

impl_foo!(ArtistList, AlbumList, SongList);

pub trait List {
    fn up(&mut self);
    fn down(&mut self);
    fn get_selected(&self) -> String;
    fn get_list(&self) -> Vec<ListItem<'static>>;
    fn first(&self) -> &String;
}

// #[derive(Debug, Clone)]
// pub struct BrowserList {
//     // list: Vec<ListItem<'a>>,
//     list: Vec<String>,
//     selection: ListState,
// }

// impl BrowserList {
//     pub fn get_name(&self) -> Option<&String> {
//         if let Some(index) = self.selection.selected() {
//             return self.list.get(index);
//         }
//         None
//     }
//     pub fn first(&self) -> &String {
//         self.list.first().unwrap()
//     }
//     pub fn get_artists(database: &Database) -> Self {
//         let mut list = Vec::new();
//         for artist in &database.artists {
//             list.push(artist.name.clone());
//         }

//         list.sort_by_key(|x| x.to_lowercase());

//         let mut selection = ListState::default();
//         selection.select(Some(0));
//         Self { list, selection }
//     }
//     pub fn get_albums_from_artist(database: &Database, name: &String) -> Self {
//         let artist = database.find_artist(&name).unwrap();
//         let mut list: Vec<String> = artist
//             .albums
//             .iter()
//             .map(|album| album.name.clone())
//             .collect();

//         list.sort_by_key(|x| x.to_lowercase());

//         let mut selection = ListState::default();
//         selection.select(Some(0));

//         Self { list, selection }
//     }
//     pub fn get_songs_from_album(database: &Database, name: &String) -> Self {
//         let album = database.find_album(&name).unwrap();

//         let mut songs = album.songs.clone();

//         songs.sort_by(|a, b| {
//             a.disc
//                 .cmp(&b.disc)
//                 .then(a.track_number.cmp(&b.track_number))
//         });

//         let list: Vec<String> = songs
//             .iter()
//             .map(|song| song.name_with_number.clone())
//             .collect();

//         let mut selection = ListState::default();
//         selection.select(Some(0));
//         Self { list, selection }
//     }
//     pub fn down(&mut self) {
//         let len = self.list.len();
//         let selection = &mut self.selection;
//         let selected = selection.selected();

//         if let Some(selected) = selected {
//             if selected + 1 > len - 1 {
//                 selection.select(Some(0));
//             } else {
//                 selection.select(Some(selected + 1));
//             }
//         }
//     }
//     pub fn up(&mut self) {
//         let len = self.list.len();
//         let selection = &mut self.selection;
//         let selected = selection.selected();

//         if let Some(selected) = selected {
//             if selected != 0 {
//                 selection.select(Some(selected - 1));
//             } else {
//                 selection.select(Some(len - 1));
//             }
//         }
//     }
// }

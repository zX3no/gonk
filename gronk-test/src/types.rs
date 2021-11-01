#[derive(Clone, Debug)]
pub struct Music {
    artists: Vec<Artist>,
    selected_artist: Artist,
}

impl Music {
    pub fn new() -> Self {
        let track_1 = Song {
            name: String::from("Panic Emoji"),
        };
        let track_2 = Song {
            name: String::from("Dayum"),
        };
        let veteran = Album {
            name: String::from("Veteren"),
            songs: vec![track_1.clone(), track_2.clone()],
            selected_song: track_1.clone(),
        };
        let lp = Album {
            name: String::from("LP!"),
            songs: vec![track_1.clone(), track_2.clone()],
            selected_song: track_1.clone(),
        };
        let jpegmafia = Artist {
            name: String::from("JPEGMAFIA"),
            albums: vec![veteran.clone(), lp.clone()],
            selected_album: veteran,
        };

        let track_1 = Song {
            name: String::from("Something Fossil"),
        };
        let track_2 = Song {
            name: String::from("Neo"),
        };
        let neowax = Album {
            name: String::from("NeoWax"),
            songs: vec![track_1.clone(), track_2.clone()],
            selected_song: track_1.clone(),
        };
        let iglooghost = Artist {
            name: String::from("Iglooghost"),
            albums: vec![neowax.clone()],
            selected_album: neowax,
        };

        Self {
            artists: vec![jpegmafia.clone(), iglooghost.clone()],
            selected_artist: iglooghost,
        }
    }
    pub fn artists_down(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_artist() {
            //try to move up
            if let Some(artist) = self.artists.get(i + 1) {
                //if we can update the selected artist
                self.selected_artist = artist.clone();
            } else {
                if let Some(artist) = self.artists.first() {
                    //if we can't reset to first artist
                    self.selected_artist = artist.clone();
                } else {
                    panic!("first returned nothing");
                }
            }
        } else {
            panic!("no selected artist?");
        }
    }
    pub fn artists_up(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_artist() {
            //try to move up
            if i == 0 {
                if let Some(artist) = self.artists.last() {
                    //if we can't reset to first artist
                    self.selected_artist = artist.clone();
                } else {
                    panic!("last returned nothing");
                }
            } else {
                if let Some(artist) = self.artists.get(i - 1) {
                    //if we can update the selected artist
                    self.selected_artist = artist.clone();
                } else {
                    panic!("could not get i - 1");
                }
            }
        } else {
            panic!("no selected artist?");
        }
    }
    pub fn albums_up(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_album() {
            //try to move up
            if i == 0 {
                if let Some(album) = self.selected_artist.albums.last() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album = album.clone();
                } else {
                    panic!("no artists?");
                }
            } else {
                if let Some(album) = self.selected_artist.albums.get(i - 1) {
                    //if we can update the selected artist
                    self.selected_artist.selected_album = album.clone();
                } else {
                    panic!("could not get i - 1");
                }
            }
        } else {
            panic!("no selected artist?");
        }
    }
    pub fn albums_down(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_album() {
            //try to move up
            if let Some(album) = self.selected_artist.albums.get(i + 1) {
                //if we can update the selected artist
                self.selected_artist.selected_album = album.clone();
            } else {
                if let Some(album) = self.selected_artist.albums.first() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album = album.clone();
                } else {
                    panic!("first returned nothing");
                }
            }
        } else {
            panic!("no selected album?");
        }
    }
    pub fn songs_up(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_song() {
            //try to move up
            if i == 0 {
                if let Some(song) = self.selected_artist.selected_album.songs.last() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album.selected_song = song.clone();
                } else {
                    //TODO: if there are no artists set to none
                    // self.selected_artist = None;
                    panic!("no artists?");
                }
            } else {
                if let Some(song) = self.selected_artist.selected_album.songs.get(i - 1) {
                    //if we can update the selected artist
                    self.selected_artist.selected_album.selected_song = song.clone();
                } else {
                    panic!("could not get i - 1");
                }
            }
        } else {
            panic!("no selected song?");
        }
    }
    pub fn songs_down(&mut self) {
        //get the current selected artist index
        if let Some(i) = self.get_selected_song() {
            //try to move up
            if let Some(song) = self.selected_artist.selected_album.songs.get(i + 1) {
                //if we can update the selected artist
                self.selected_artist.selected_album.selected_song = song.clone();
            } else {
                if let Some(song) = self.selected_artist.selected_album.songs.first() {
                    //if we can't reset to first artist
                    self.selected_artist.selected_album.selected_song = song.clone();
                } else {
                    panic!("first returned nothing");
                }
            }
        } else {
            panic!("no selected song?");
        }
    }

    pub fn artist_names(&self) -> Vec<String> {
        self.artists.iter().map(|a| a.name.clone()).collect()
    }
    pub fn album_names(&self) -> Vec<String> {
        self.selected_artist
            .albums
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }
    pub fn song_names(&self) -> Vec<String> {
        self.selected_artist
            .selected_album
            .songs
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }
    pub fn get_selected_artist(&self) -> Option<usize> {
        for (i, artist) in self.artists.iter().enumerate() {
            if artist == &self.selected_artist {
                return Some(i);
            }
        }
        None
    }
    pub fn get_selected_album(&self) -> Option<usize> {
        for (i, album) in self.selected_artist.albums.iter().enumerate() {
            if album == &self.selected_artist.selected_album {
                return Some(i);
            }
        }
        None
    }
    pub fn get_selected_song(&self) -> Option<usize> {
        for (i, album) in self.selected_artist.selected_album.songs.iter().enumerate() {
            if album == &self.selected_artist.selected_album.selected_song {
                return Some(i);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
struct Artist {
    name: String,
    albums: Vec<Album>,
    selected_album: Album,
}

impl PartialEq for Artist {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.albums == other.albums
        // && self.selected_album == other.selected_album
    }
}

#[derive(Clone, Debug)]
struct Album {
    name: String,
    songs: Vec<Song>,
    selected_song: Song,
}

impl PartialEq for Album {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.songs == other.songs
        // && self.selected_song == other.selected_song
    }
}

#[derive(Clone, Debug)]
struct Song {
    name: String,
}
impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
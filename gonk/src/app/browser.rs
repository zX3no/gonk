use crate::widgets::{List, ListItem, ListState};
use crate::{sqlite, Frame};
use gonk_player::{Index, Song};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders},
};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Artist,
    Album,
    Song,
}

impl Mode {
    pub fn right(&mut self) {
        match self {
            Mode::Artist => *self = Mode::Album,
            Mode::Album => *self = Mode::Song,
            Mode::Song => (),
        }
    }
    pub fn left(&mut self) {
        match self {
            Mode::Artist => (),
            Mode::Album => *self = Mode::Artist,
            Mode::Song => *self = Mode::Album,
        }
    }
}

pub struct BrowserSong {
    name: String,
    id: usize,
}

pub struct Browser {
    artists: Index<String>,
    albums: Index<String>,
    songs: Index<BrowserSong>,
    pub mode: Mode,
}

impl Browser {
    pub fn new() -> Self {
        let artists = Index::new(sqlite::get_all_artists(), Some(0));

        let (albums, songs) = if let Some(first_artist) = artists.selected() {
            let albums = Index::new(sqlite::get_all_albums_by_artist(first_artist), Some(0));

            if let Some(first_album) = albums.selected() {
                let songs = sqlite::get_all_songs_from_album(first_album, first_artist)
                    .into_iter()
                    .map(|song| BrowserSong {
                        name: format!("{}. {}", song.number, song.name),
                        id: song.id.unwrap(),
                    })
                    .collect();
                (albums, Index::new(songs, Some(0)))
            } else {
                (albums, Index::default())
            }
        } else {
            (Index::default(), Index::default())
        };

        Self {
            artists,
            albums,
            songs,
            mode: Mode::Artist,
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            Mode::Artist => self.artists.up(),
            Mode::Album => self.albums.up(),
            Mode::Song => self.songs.up(),
        }
        self.update_browser();
    }
    pub fn down(&mut self) {
        match self.mode {
            Mode::Artist => self.artists.down(),
            Mode::Album => self.albums.down(),
            Mode::Song => self.songs.down(),
        }
        self.update_browser();
    }
    pub fn update_browser(&mut self) {
        match self.mode {
            Mode::Artist => self.update_albums(),
            Mode::Album => self.update_songs(),
            Mode::Song => (),
        }
    }
    pub fn update_albums(&mut self) {
        //Update the album based on artist selection
        if let Some(artist) = self.artists.selected() {
            self.albums = Index::new(sqlite::get_all_albums_by_artist(artist), Some(0));
            self.update_songs();
        }
    }
    pub fn update_songs(&mut self) {
        if let Some(artist) = self.artists.selected() {
            if let Some(album) = self.albums.selected() {
                let songs = sqlite::get_all_songs_from_album(album, artist)
                    .into_iter()
                    .map(|song| BrowserSong {
                        name: format!("{}. {}", song.number, song.name),
                        id: song.id.unwrap(),
                    })
                    .collect();
                self.songs = Index::new(songs, Some(0));
            }
        }
    }
    pub fn right(&mut self) {
        self.mode.right();
    }
    pub fn left(&mut self) {
        self.mode.left();
    }
    pub fn on_enter(&self) -> Vec<Song> {
        if let Some(artist) = self.artists.selected() {
            if let Some(album) = self.albums.selected() {
                if let Some(song) = self.songs.selected() {
                    return match self.mode {
                        Mode::Artist => sqlite::get_songs_by_artist(artist),
                        Mode::Album => sqlite::get_all_songs_from_album(album, artist),
                        Mode::Song => sqlite::get_songs(&[song.id]),
                    };
                }
            }
        }
        Vec::new()
    }
    pub fn refresh(&mut self) {
        self.mode = Mode::Artist;

        self.artists = Index::new(sqlite::get_all_artists(), Some(0));
        self.albums = Index::default();
        self.songs = Index::default();

        self.update_albums();
    }
}

impl Browser {
    fn list<'a>(title: &'static str, content: &'a [ListItem], use_symbol: bool) -> List<'a> {
        let list = List::new(content.to_vec())
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White));

        if use_symbol {
            list.highlight_symbol(">")
        } else {
            list.highlight_symbol("")
        }
    }
    pub fn draw(&self, area: Rect, f: &mut Frame) {
        let size = area.width / 3;
        let rem = area.width % 3;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(size),
                Constraint::Length(size),
                Constraint::Length(size + rem),
            ])
            .split(area);

        let a: Vec<ListItem> = self
            .artists
            .data
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let b: Vec<ListItem> = self
            .albums
            .data
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let c: Vec<ListItem> = self
            .songs
            .data
            .iter()
            .map(|song| ListItem::new(song.name.as_str()))
            .collect();

        let artists = Browser::list("─Aritst", &a, self.mode == Mode::Artist);
        let albums = Browser::list("─Album", &b, self.mode == Mode::Album);
        let songs = Browser::list("─Song", &c, self.mode == Mode::Song);

        f.render_stateful_widget(
            artists,
            chunks[0],
            &mut ListState::new(self.artists.index()),
        );
        f.render_stateful_widget(albums, chunks[1], &mut ListState::new(self.albums.index()));
        f.render_stateful_widget(songs, chunks[2], &mut ListState::new(self.songs.index()));
    }
}

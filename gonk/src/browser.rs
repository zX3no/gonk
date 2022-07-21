use crate::widgets::{List, ListItem, ListState};
use crate::{Frame, Input};
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
        let artists = Index::new(gonk_database::artists(), Some(0));

        let (albums, songs) = if let Some(first_artist) = artists.selected() {
            let albums = Index::new(gonk_database::albums_by_artist(first_artist), Some(0));

            if let Some(first_album) = albums.selected() {
                let songs = gonk_database::songs_from_album(first_album, first_artist)
                    .into_iter()
                    .map(|song| BrowserSong {
                        name: format!("{}. {}", song.number, song.name),
                        id: song.id,
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
}

impl Input for Browser {
    fn up(&mut self) {
        match self.mode {
            Mode::Artist => self.artists.up(),
            Mode::Album => self.albums.up(),
            Mode::Song => self.songs.up(),
        }
        update_browser(self);
    }

    fn down(&mut self) {
        match self.mode {
            Mode::Artist => self.artists.down(),
            Mode::Album => self.albums.down(),
            Mode::Song => self.songs.down(),
        }
        update_browser(self);
    }

    fn left(&mut self) {
        match self.mode {
            Mode::Artist => (),
            Mode::Album => self.mode = Mode::Artist,
            Mode::Song => self.mode = Mode::Album,
        }
    }

    fn right(&mut self) {
        match self.mode {
            Mode::Artist => self.mode = Mode::Album,
            Mode::Album => self.mode = Mode::Song,
            Mode::Song => (),
        }
    }
}

pub fn refresh(browser: &mut Browser) {
    browser.mode = Mode::Artist;

    browser.artists = Index::new(gonk_database::artists(), Some(0));
    browser.albums = Index::default();
    browser.songs = Index::default();

    update_albums(browser);
}

pub fn update_browser(browser: &mut Browser) {
    match browser.mode {
        Mode::Artist => update_albums(browser),
        Mode::Album => update_songs(browser),
        Mode::Song => (),
    }
}

pub fn update_albums(browser: &mut Browser) {
    //Update the album based on artist selection
    if let Some(artist) = browser.artists.selected() {
        browser.albums = Index::new(gonk_database::albums_by_artist(artist), Some(0));
        update_songs(browser);
    }
}

pub fn update_songs(browser: &mut Browser) {
    if let Some(artist) = browser.artists.selected() {
        if let Some(album) = browser.albums.selected() {
            let songs = gonk_database::songs_from_album(album, artist)
                .into_iter()
                .map(|song| BrowserSong {
                    name: format!("{}. {}", song.number, song.name),
                    id: song.id,
                })
                .collect();
            browser.songs = Index::new(songs, Some(0));
        }
    }
}

pub fn get_selected(browser: &Browser) -> Vec<Song> {
    if let Some(artist) = browser.artists.selected() {
        if let Some(album) = browser.albums.selected() {
            if let Some(song) = browser.songs.selected() {
                return match browser.mode {
                    Mode::Artist => gonk_database::songs_by_artist(artist),
                    Mode::Album => gonk_database::songs_from_album(album, artist),
                    Mode::Song => gonk_database::ids(&[song.id]),
                };
            }
        }
    }
    Vec::new()
}

pub fn draw(browser: &Browser, area: Rect, f: &mut Frame) {
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

    let a: Vec<ListItem> = browser
        .artists
        .data
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let b: Vec<ListItem> = browser
        .albums
        .data
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let c: Vec<ListItem> = browser
        .songs
        .data
        .iter()
        .map(|song| ListItem::new(song.name.as_str()))
        .collect();

    let artists = list("─Aritst", &a, browser.mode == Mode::Artist);
    let albums = list("─Album", &b, browser.mode == Mode::Album);
    let songs = list("─Song", &c, browser.mode == Mode::Song);

    f.render_stateful_widget(
        artists,
        chunks[0],
        &mut ListState::new(browser.artists.index()),
    );
    f.render_stateful_widget(
        albums,
        chunks[1],
        &mut ListState::new(browser.albums.index()),
    );
    f.render_stateful_widget(songs, chunks[2], &mut ListState::new(browser.songs.index()));
}

fn list<'a>(title: &'static str, content: &'a [ListItem], use_symbol: bool) -> List<'a> {
    let list = List::new(content)
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

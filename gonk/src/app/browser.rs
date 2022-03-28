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

pub struct Browser {
    db: Database,
    artists: Index<String>,
    albums: Index<String>,
    songs: Index<(u16, String)>,
    pub mode: BrowserMode,
    pub is_busy: bool,
}

impl Browser {
    pub fn new() -> Self {
        let db = Database::new().unwrap();
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
            BrowserMode::Artist => self.db.artist(artist),
            BrowserMode::Album => self.db.album(artist, album),
            BrowserMode::Song => self.db.get_song(artist, album, song),
        }
    }
    pub fn refresh(&mut self) {
        self.mode = BrowserMode::Artist;
        self.albums = Index::default();
        self.songs = Index::default();

        self.artists = Index::new(self.db.artists(), Some(0));

        if let Some(first_artist) = self.artists.selected() {
            self.albums = Index::new(self.db.albums_by_artist(first_artist), Some(0));
        }

        if let Some(first_artist) = self.artists.selected() {
            if let Some(first_album) = self.albums.selected() {
                self.songs =
                    Index::new(self.db.songs_from_album(first_artist, first_album), Some(0));
            }
        }
    }
}

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Spans,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
impl Browser {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        self.draw_browser(f);

        if self.is_busy {
            self.draw_popup(f);
        }
    }
    pub fn draw_browser<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ]
                .as_ref(),
            )
            .split(area);

        let a: Vec<_> = self
            .artists
            .data
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let b: Vec<_> = self
            .albums
            .data
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let c: Vec<_> = self
            .songs
            .data
            .iter()
            .map(|song| ListItem::new(format!("{}. {}", song.0, song.1)))
            .collect();

        let artists = List::new(a)
            .block(
                Block::default()
                    .title("─Aritst")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut artist_state = ListState::default();
        artist_state.select(self.artists.index);

        let albums = List::new(b)
            .block(
                Block::default()
                    .title("─Album")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut album_state = ListState::default();
        album_state.select(self.albums.index);

        let songs = List::new(c)
            .block(
                Block::default()
                    .title("─Song")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut song_state = ListState::default();
        song_state.select(self.songs.index);

        //TODO: better way of doing this?
        match self.mode {
            BrowserMode::Artist => {
                album_state.select(None);
                song_state.select(None);
            }
            BrowserMode::Album => {
                artist_state.select(None);
                song_state.select(None);
            }
            BrowserMode::Song => {
                artist_state.select(None);
                album_state.select(None);
            }
        }

        f.render_stateful_widget(artists, chunks[0], &mut artist_state);
        f.render_stateful_widget(albums, chunks[1], &mut album_state);
        f.render_stateful_widget(songs, chunks[2], &mut song_state);
    }

    //TODO: change to small text in bottom right
    pub fn draw_popup<B: Backend>(&self, f: &mut Frame<B>) {
        let mut area = f.size();

        if (area.width / 2) < 14 || (area.height / 2) < 3 {
            return;
        }

        area.x = (area.width / 2) - 7;
        if (area.width / 2) % 2 == 0 {
            area.y = (area.height / 2) - 3;
        } else {
            area.y = (area.height / 2) - 2;
        }
        area.width = 14;
        area.height = 3;

        let text = vec![Spans::from("Scanning...")];

        let p = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Center);

        f.render_widget(p, area);
    }
}

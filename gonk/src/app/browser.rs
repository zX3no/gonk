use super::DB;
use crate::widget::{List, ListItem, ListState};
use gonk_types::{Index, Song};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Spans,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub enum Mode {
    Artist,
    Album,
    Song,
}

impl Mode {
    pub fn next(&mut self) {
        match self {
            Mode::Artist => *self = Mode::Album,
            Mode::Album => *self = Mode::Song,
            Mode::Song => (),
        }
    }
    pub fn prev(&mut self) {
        match self {
            Mode::Artist => (),
            Mode::Album => *self = Mode::Artist,
            Mode::Song => *self = Mode::Album,
        }
    }
}

pub struct Browser {
    artists: Index<String>,
    albums: Index<String>,
    //TODO: change to just a string?
    songs: Index<(u64, String)>,
    pub mode: Mode,
}

impl Browser {
    pub fn new() -> Self {
        optick::event!("new browser");
        let artists = Index::new(DB.get_all_artists(), Some(0));

        let (albums, songs) = if let Some(first_artist) = artists.selected() {
            let albums = Index::new(DB.get_all_albums_by_artist(first_artist), Some(0));

            if let Some(first_album) = albums.selected() {
                let songs = DB
                    .get_songs_from_album(first_album, first_artist)
                    .iter()
                    .map(|song| (song.number, song.name.clone()))
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
            self.albums = Index::new(DB.get_all_albums_by_artist(artist), Some(0));
            self.update_songs();
        }
    }
    pub fn update_songs(&mut self) {
        if let Some(artist) = self.artists.selected() {
            if let Some(album) = self.albums.selected() {
                let songs = DB
                    .get_songs_from_album(album, artist)
                    .iter()
                    .map(|song| (song.number, song.name.clone()))
                    .collect();
                self.songs = Index::new(songs, Some(0));
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
        if let Some(artist) = self.artists.selected() {
            if let Some(album) = self.albums.selected() {
                if let Some(song) = self.songs.selected() {
                    return match self.mode {
                        Mode::Artist => DB.get_songs_by_artist(artist),
                        Mode::Album => DB.get_songs_from_album(album, artist),
                        Mode::Song => DB.get_song(song, album, artist),
                    };
                }
            }
        }
        Vec::new()
    }
    pub fn refresh(&mut self) {
        self.mode = Mode::Artist;

        self.artists = Index::new(DB.get_all_artists(), Some(0));
        self.albums = Index::default();
        self.songs = Index::default();

        self.update_albums();
    }
}

impl Browser {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, busy: bool) {
        optick::event!("draw browser");
        self.draw_browser(f);
        if busy {
            Browser::draw_popup(f);
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

        let mut artist_state = ListState::new(self.artists.index);

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

        let mut album_state = ListState::new(self.albums.index);

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

        let mut song_state = ListState::new(self.songs.index);

        //TODO: better way of doing this?
        match self.mode {
            Mode::Artist => {
                album_state.select(None);
                song_state.select(None);
            }
            Mode::Album => {
                artist_state.select(None);
                song_state.select(None);
            }
            Mode::Song => {
                artist_state.select(None);
                album_state.select(None);
            }
        }

        f.render_stateful_widget(artists, chunks[0], &mut artist_state);
        f.render_stateful_widget(albums, chunks[1], &mut album_state);
        f.render_stateful_widget(songs, chunks[2], &mut song_state);
    }

    //TODO: change to small text in bottom right
    pub fn draw_popup<B: Backend>(f: &mut Frame<B>) {
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

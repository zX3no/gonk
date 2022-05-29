use crate::widgets::{List, ListItem, ListState};
use gonk_core::{sqlite, Index, Song};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
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
    songs: Index<(u64, String)>,
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
                    .map(|song| (song.number, song.name))
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
                    .map(|song| (song.number, song.name))
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
                        Mode::Artist => sqlite::get_songs_by_artist(artist),
                        Mode::Album => sqlite::get_all_songs_from_album(album, artist),
                        Mode::Song => sqlite::get_song(song, album, artist),
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
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, busy: bool) {
        self.draw_browser(f);
        if busy {
            Browser::draw_popup(f);
        }
    }
    fn list<'a>(title: &'static str, content: &'a [ListItem]) -> List<'a> {
        List::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">")
    }
    pub fn draw_browser<B: Backend>(&self, f: &mut Frame<B>) {
        const EMPTY_LIST: ListState = ListState::new(None);

        let area = f.size();
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
            .map(|song| ListItem::new(format!("{}. {}", song.0, song.1)))
            .collect();

        let artists = Browser::list("─Aritst", &a);
        let albums = Browser::list("─Album", &b);
        let songs = Browser::list("─Song", &c);

        let (mut artist, mut album, mut song) = match self.mode {
            Mode::Artist => (ListState::new(self.artists.index()), EMPTY_LIST, EMPTY_LIST),
            Mode::Album => (EMPTY_LIST, ListState::new(self.albums.index()), EMPTY_LIST),
            Mode::Song => (EMPTY_LIST, EMPTY_LIST, ListState::new(self.songs.index())),
        };

        f.render_stateful_widget(artists, chunks[0], &mut artist);
        f.render_stateful_widget(albums, chunks[1], &mut album);
        f.render_stateful_widget(songs, chunks[2], &mut song);
    }

    //TODO: change to small text in bottom right
    //This bar should show in all pages but the queue.
    //It will show what the current song is, how much is left and the volume.
    pub fn draw_popup<B: Backend>(f: &mut Frame<B>) {
        const POPUP_WIDTH: u16 = 14;
        const POPUP_HEIGHT: u16 = 3;

        let area = f.size();
        let width = area.width / 2;
        let height = area.height / 2;

        if width < POPUP_WIDTH || height < POPUP_HEIGHT {
            return;
        }

        let mut rect = area.inner(&Margin {
            vertical: height - (POPUP_HEIGHT / 2),
            horizontal: width - (POPUP_WIDTH / 2),
        });

        rect.width = POPUP_WIDTH;
        rect.height = POPUP_HEIGHT;

        f.render_widget(
            Paragraph::new(Spans::from("Scanning..."))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .alignment(Alignment::Center),
            rect,
        );
    }
}

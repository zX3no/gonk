use super::{Mode as AppMode, COLORS};
use crate::index::Index;
use crossterm::event::KeyModifiers;
use gonk_database::Database;
use lib::{Album, Artist, Engine, Item, SearchItem, Song};
use rodio::Player;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

mod lib;

pub enum Mode {
    Search,
    Select,
}

pub struct Search {
    db: Database,
    query: String,
    prev_query: String,
    mode: Mode,
    results: Index<Item>,
    engine: Engine<Item>,
}

impl Search {
    pub fn update_engine(&mut self) {
        let songs = self.db.get_songs();
        let artists = self.db.artists();
        let albums = self.db.albums();
        self.engine = Engine::default();

        for (id, song) in songs {
            self.engine.push(Item::Song(Song::new(
                id,
                song.name,
                song.album,
                song.artist,
            )));
        }

        for (album, artist) in albums {
            self.engine.push(Item::Album(Album::new(album, artist)));
        }

        for artist in artists {
            self.engine.push(Item::Artist(Artist::new(artist)));
        }
    }
    pub fn new() -> Self {
        let mut s = Self {
            engine: Engine::default(),
            db: Database::new().unwrap(),
            query: String::new(),
            prev_query: String::new(),
            mode: Mode::Search,
            results: Index::default(),
        };
        //TODO: this is dumb
        s.update_engine();
        s
    }
    pub fn update_search(&mut self) {
        let query = &self.query.to_lowercase();
        let mut results = Vec::new();

        for item in &self.engine.data {
            let acc = if let Some(artist) = item.artist() {
                if let Some(album) = item.album() {
                    if let Some((_, song)) = item.song() {
                        strsim::jaro_winkler(query, &song.to_lowercase())
                    } else {
                        strsim::jaro_winkler(query, &album.to_lowercase())
                    }
                } else {
                    strsim::jaro_winkler(query, &artist.to_lowercase())
                }
            } else {
                panic!("Invalid search item")
            };

            if acc > 0.75 {
                results.push((item, acc));
            }
        }
        //TODO: sort self titled albums bellow artists
        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        self.results.data = results.into_iter().map(|(item, _)| item.clone()).collect();
    }
    pub fn on_key(&mut self, c: char) {
        if let Mode::Search = &self.mode {
            self.prev_query = self.query.clone();
            self.query.push(c);
        } else {
            match c {
                'k' => self.results.up(),
                'j' => self.results.down(),
                _ => (),
            }
        }
    }
    pub fn on_tab(&mut self) {
        match self.mode {
            Mode::Search => {
                self.results.select(None);
                self.query.clear();
            }
            Mode::Select => (),
        }
    }
    pub fn up(&mut self) {
        self.results.up();
    }
    pub fn down(&mut self) {
        self.results.down();
    }
    pub fn on_backspace(&mut self, modifiers: KeyModifiers) {
        match self.mode {
            Mode::Search => {
                if modifiers == KeyModifiers::CONTROL {
                    self.query.clear();
                } else {
                    self.query.pop();
                }
            }
            Mode::Select => {
                self.results.select(None);
                self.mode = Mode::Search;
            }
        }
    }
    pub fn has_query_changed(&mut self) -> bool {
        if self.query == self.prev_query {
            false
        } else {
            self.prev_query = self.query.clone();
            true
        }
    }
    pub fn on_escape(&mut self, mode: &mut AppMode) {
        match self.mode {
            Mode::Search => {
                if let Mode::Search = self.mode {
                    self.query.clear();
                    *mode = AppMode::Queue;
                }
            }
            Mode::Select => {
                self.mode = Mode::Search;
                self.results.select(None);
            }
        }
    }
    pub fn on_enter(&mut self, player: &mut Player) {
        match self.mode {
            Mode::Search => {
                if !self.results.is_empty() {
                    self.mode = Mode::Select;
                    self.results.select(Some(0));
                }
            }
            Mode::Select => {
                if let Some(item) = self.results.selected() {
                    let songs = if let Some((id, _)) = item.song() {
                        vec![self.db.get_song_from_id(id)]
                    } else if let Some(artist) = item.artist() {
                        if let Some(album) = item.album() {
                            self.db.album(artist, album)
                        } else {
                            self.db.artist(artist)
                        }
                    } else {
                        panic!("Invalid search item");
                    };

                    player.add_songs(songs);
                }
            }
        }
    }
}

impl Search {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                ]
                .as_ref(),
            )
            .split(area);

        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(v[1]);

        self.draw_textbox(f, v[0]);

        let item = if self.results.selected().is_some() {
            self.results.selected()
        } else {
            self.results.data.first()
        };

        if let Some(item) = item {
            if let Some(artist) = item.artist() {
                if let Some(album) = item.album() {
                    if let Some(song) = item.song() {
                        //song
                        Search::song(f, song, album, artist, h[0]);
                        self.album(f, album, artist, h[1]);
                    } else {
                        //album
                        self.album(f, album, artist, h[0]);
                        self.artist(f, artist, h[1]);
                    }
                } else {
                    //artist
                    let albums = self.db.albums_by_artist(artist);

                    self.artist(f, artist, h[0]);

                    let h_split = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
                        )
                        .split(h[1]);

                    //draw the first two albums
                    for (i, area) in h_split.iter().enumerate() {
                        if let Some(album) = albums.get(i) {
                            self.album(f, album, artist, *area);
                        }
                    }
                }
            }
            self.draw_results(f, v[2]);
        } else {
            self.draw_results(f, v[1].union(v[2]));
        }

        self.update_cursor(f);
    }
    fn song<B: Backend>(
        f: &mut Frame<B>,
        (_, song): (usize, &String),
        album: &str,
        artist: &str,
        area: Rect,
    ) {
        let song_table = Table::new(vec![
            Row::new(vec![Spans::from(Span::raw(album))]),
            Row::new(vec![Spans::from(Span::raw(artist))]),
        ])
        .header(
            Row::new(vec![Span::styled(
                format!("{} ", song),
                Style::default().add_modifier(Modifier::ITALIC),
            )])
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Song"),
        )
        .widths(&[Constraint::Percentage(100)]);

        f.render_widget(song_table, area);
    }
    fn album<B: Backend>(&self, f: &mut Frame<B>, album: &str, artist: &str, area: Rect) {
        let cells: Vec<_> = self
            .db
            .album(artist, album)
            .iter()
            .map(|song| Row::new(vec![Cell::from(format!("{}. {}", song.number, song.name))]))
            .collect();

        let album_table = Table::new(cells)
            .header(
                Row::new(vec![Cell::from(Span::styled(
                    format!("{} ", album),
                    Style::default().add_modifier(Modifier::ITALIC),
                ))])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Album"),
            )
            .widths(&[Constraint::Percentage(100)]);

        f.render_widget(album_table, area);
    }
    fn artist<B: Backend>(&self, f: &mut Frame<B>, artist: &str, area: Rect) {
        let albums = self.db.albums_by_artist(artist);
        let cells: Vec<_> = albums
            .iter()
            .map(|album| Row::new(vec![Cell::from(Span::raw(album))]))
            .collect();

        let artist_table = Table::new(cells)
            .header(
                Row::new(vec![Cell::from(Span::styled(
                    format!("{} ", artist),
                    Style::default().add_modifier(Modifier::ITALIC),
                ))])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Artist"),
            )
            .widths(&[Constraint::Percentage(100)]);

        f.render_widget(artist_table, area);
    }
    fn draw_results<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let get_cell = |item: &dyn SearchItem, modifier: Modifier| -> Row {
            if let Some(artist) = item.artist() {
                if let Some(album) = item.album() {
                    if let Some((id, _)) = item.song() {
                        //song
                        let song = self.db.get_song_from_id(id);
                        Row::new(vec![
                            Cell::from(song.name)
                                .style(Style::default().fg(COLORS.title).add_modifier(modifier)),
                            Cell::from(song.album.clone())
                                .style(Style::default().fg(COLORS.album).add_modifier(modifier)),
                            Cell::from(song.artist)
                                .style(Style::default().fg(COLORS.artist).add_modifier(modifier)),
                        ])
                    } else {
                        //album
                        Row::new(vec![
                            Cell::from(format!("{} - Album", album))
                                .style(Style::default().fg(COLORS.title).add_modifier(modifier)),
                            Cell::from("")
                                .style(Style::default().fg(COLORS.album).add_modifier(modifier)),
                            Cell::from(artist.clone())
                                .style(Style::default().fg(COLORS.artist).add_modifier(modifier)),
                        ])
                    }
                } else {
                    Row::new(vec![
                        Cell::from(format!("{} - Artist", artist))
                            .style(Style::default().fg(COLORS.title).add_modifier(modifier)),
                        Cell::from("")
                            .style(Style::default().fg(COLORS.album).add_modifier(modifier)),
                        Cell::from("")
                            .style(Style::default().fg(COLORS.artist).add_modifier(modifier)),
                    ])
                    //artist
                }
            } else {
                panic!();
            }
        };

        let selected = &self.results.index;
        let rows: Vec<_> = self
            .results
            .data
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if let Some(s) = selected {
                    if s == &i {
                        return get_cell(item, Modifier::ITALIC);
                    }
                } else if i == 0 {
                    return get_cell(item, Modifier::ITALIC);
                }
                get_cell(item, Modifier::empty())
            })
            .collect();

        let italic = Style::default().add_modifier(Modifier::ITALIC);
        let t = Table::new(rows)
            .header(
                Row::new(vec![
                    Cell::from("Name").style(italic),
                    Cell::from("Album").style(italic),
                    Cell::from("Artist").style(italic),
                ])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .widths(&[
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ])
            .highlight_symbol("> ");

        let mut state = TableState::default();
        state.select(self.results.index);

        f.render_stateful_widget(t, area, &mut state);
    }
    fn draw_textbox<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let p = Paragraph::new(self.query.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left);
        f.render_widget(p, area);
    }
    fn update_cursor<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();
        //Move the cursor position when typing
        if let Mode::Search = self.mode {
            if self.results.is_none() && self.query.is_empty() {
                f.set_cursor(1, 1);
            } else {
                //TODO: casting to u16 could fail
                let mut len = self.query.len() as u16;
                if len > area.width {
                    len = area.width;
                }
                f.set_cursor(len + 1, 1);
            }
        }
    }
}

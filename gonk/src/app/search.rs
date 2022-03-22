use crate::index::Index;
use crossterm::event::KeyModifiers;
use gonk_database::{Colors, Database};
use gonk_search::{ItemType, SearchEngine, SearchItem};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use super::{queue::Queue, Mode};

pub enum SearchMode {
    Search,
    Select,
}

impl SearchMode {
    pub fn next(&mut self) {
        match self {
            SearchMode::Search => *self = SearchMode::Select,
            SearchMode::Select => *self = SearchMode::Search,
        }
    }
}

pub struct Search<'a> {
    db: &'a Database,
    engine: SearchEngine,
    query: String,
    prev_query: String,
    mode: SearchMode,
    results: Index<SearchItem>,
}

impl<'a> Search<'a> {
    fn new_engine(db: &Database) -> SearchEngine {
        let mut engine = SearchEngine::default();

        let songs = db.get_songs();
        let artists = db.artists();
        let albums = db.albums();

        let songs: Vec<_> = songs
            .iter()
            .map(|(song, id)| SearchItem::song(&song.name, *id))
            .collect();

        let albums: Vec<_> = albums
            .iter()
            .map(|(name, artist)| SearchItem::album(name, artist))
            .collect();

        let artists: Vec<_> = artists
            .iter()
            .map(|name| SearchItem::artist(name))
            .collect();

        engine.insert_vec(songs);
        engine.insert_vec(albums);
        engine.insert_vec(artists);

        engine
    }
    pub fn new(db: &'a Database) -> Self {
        Self {
            engine: Search::new_engine(db),
            db,
            query: String::new(),
            prev_query: String::new(),
            results: Index::default(),
            mode: SearchMode::Search,
        }
    }
    pub fn update_search(&mut self) {
        self.results.data = self.engine.search(&self.query);
    }
    pub fn on_key(&mut self, c: char) {
        if let SearchMode::Search = &self.mode {
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
    pub fn refresh(&mut self) {
        self.engine = Search::new_engine(self.db);
    }
    pub fn on_tab(&mut self) {
        match self.mode {
            SearchMode::Search => {
                self.results.select(None);
                self.query.clear();
            }
            SearchMode::Select => (),
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
            SearchMode::Search => {
                if modifiers == KeyModifiers::CONTROL {
                    self.query.clear();
                } else {
                    self.query.pop();
                }
            }
            SearchMode::Select => {
                self.results.select(None);
                self.mode.next();
            }
        }
    }
    pub fn has_query_changed(&mut self) -> bool {
        if self.query != self.prev_query {
            self.prev_query = self.query.clone();
            true
        } else {
            false
        }
    }
    pub fn on_escape(&mut self, mode: &mut Mode) {
        match self.mode {
            SearchMode::Search => {
                if let SearchMode::Search = self.mode {
                    self.query.clear();
                    *mode = Mode::Queue;
                }
            }
            SearchMode::Select => {
                self.mode.next();
                self.results.select(None);
            }
        }
    }
    pub fn on_enter(&mut self, queue: &mut Queue) {
        match self.mode {
            SearchMode::Search => {
                if !self.results.is_empty() {
                    self.mode.next();
                    self.results.select(Some(0));
                }
            }
            SearchMode::Select => {
                if let Some(item) = self.results.selected() {
                    let songs = match item.item_type {
                        ItemType::Song => vec![self.db.get_song_from_id(item.song_id.unwrap())],
                        ItemType::Album => self
                            .db
                            .album(item.album_artist.as_ref().unwrap(), &item.name),
                        ItemType::Artist => self.db.artist(&item.name),
                    };
                    queue.add(songs);
                }
            }
        }
    }
}

impl<'a> Search<'a> {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, colors: &Colors) {
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
            match item.item_type {
                ItemType::Song => self.draw_song(f, h, item),
                ItemType::Album => self.draw_album(f, h, item),
                ItemType::Artist => self.draw_artist(f, h, &item.name),
            }
            self.draw_results(f, v[2], colors);
        } else {
            self.draw_results(f, v[1].union(v[2]), colors);
        }

        self.update_cursor(f);
    }
    fn draw_song<B: Backend>(&self, f: &mut Frame<B>, h: Vec<Rect>, song: &SearchItem) {
        let id = song.song_id.unwrap();
        let s = self.db.get_song_from_id(id);

        let song_table = Table::new(vec![
            Row::new(vec![Spans::from(Span::raw(&s.album))]),
            Row::new(vec![Spans::from(Span::raw(&s.artist))]),
        ])
        .header(
            Row::new(vec![Span::styled(
                format!("{} ", &s.name),
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
        .widths(&[Constraint::Percentage(100)])
        .highlight_symbol("> ");

        f.render_widget(song_table, h[0]);

        //TODO: Highlight the selected song in the album
        let album = self.db.album(&s.artist, &s.album);
        let rows: Vec<_> = album
            .iter()
            .map(|song| Row::new(vec![Spans::from(format!("{}. {}", song.number, song.name))]))
            .collect();

        let album_table = Table::new(rows)
            .header(
                Row::new(vec![Spans::from(Span::styled(
                    format!("{} ", s.album),
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
            .widths(&[Constraint::Percentage(100)])
            .highlight_symbol("> ");

        f.render_widget(album_table, h[1]);
    }
    fn draw_album<B: Backend>(&self, f: &mut Frame<B>, h: Vec<Rect>, album: &SearchItem) {
        let artist = album.album_artist.as_ref().unwrap();
        let a = self.db.album(artist, &album.name);

        let cells: Vec<_> = a
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
            .widths(&[Constraint::Percentage(100)])
            .highlight_symbol("> ");

        f.render_widget(album_table, h[0]);

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
            .widths(&[Constraint::Percentage(100)])
            .highlight_symbol("> ");

        f.render_widget(artist_table, h[1]);
    }
    fn draw_artist<B: Backend>(&self, f: &mut Frame<B>, h: Vec<Rect>, artist: &String) {
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
            .widths(&[Constraint::Percentage(100)])
            .highlight_symbol("> ");

        f.render_widget(artist_table, h[0]);

        let h_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(h[1]);

        for (i, area) in h_split.iter().enumerate() {
            if let Some(album) = albums.get(i) {
                let a = self.db.album(artist, album);

                let cells: Vec<_> = a
                    .iter()
                    .map(|song| {
                        Row::new(vec![Cell::from(format!("{}. {}", song.number, song.name))])
                    })
                    .collect();

                let album = Table::new(cells)
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
                            .title("Artist"),
                    )
                    .widths(&[Constraint::Percentage(100)])
                    .highlight_symbol("> ");

                f.render_widget(album, *area);
            }
        }
    }
    fn draw_results<B: Backend>(&self, f: &mut Frame<B>, area: Rect, colors: &Colors) {
        let get_cell = |item: &SearchItem, modifier: Modifier| -> Row {
            match item.item_type {
                ItemType::Song => {
                    let song = self.db.get_song_from_id(item.song_id.unwrap());
                    Row::new(vec![
                        Cell::from(song.name)
                            .style(Style::default().fg(colors.title).add_modifier(modifier)),
                        Cell::from(song.album.to_owned())
                            .style(Style::default().fg(colors.album).add_modifier(modifier)),
                        Cell::from(song.artist)
                            .style(Style::default().fg(colors.artist).add_modifier(modifier)),
                    ])
                }
                ItemType::Album => Row::new(vec![
                    Cell::from(format!("{} - Album", item.name))
                        .style(Style::default().fg(colors.title).add_modifier(modifier)),
                    Cell::from("").style(Style::default().fg(colors.album).add_modifier(modifier)),
                    Cell::from(item.album_artist.as_ref().unwrap().clone())
                        .style(Style::default().fg(colors.artist).add_modifier(modifier)),
                ]),
                ItemType::Artist => Row::new(vec![
                    Cell::from(format!("{} - Artist", item.name))
                        .style(Style::default().fg(colors.title).add_modifier(modifier)),
                    Cell::from("").style(Style::default().fg(colors.album).add_modifier(modifier)),
                    Cell::from("").style(Style::default().fg(colors.artist).add_modifier(modifier)),
                ]),
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
        if let SearchMode::Search = self.mode {
            if self.results.is_none() && self.query.is_empty() {
                f.set_cursor(1, 1);
            } else {
                let mut len = self.query.len() as u16;
                if len > area.width {
                    len = area.width;
                }
                f.set_cursor(len + 1, 1);
            }
        }
    }
}

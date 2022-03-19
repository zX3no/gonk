use crate::index::Index;
use crossterm::event::KeyModifiers;
use gonk_database::Database;
use gonk_search::{ItemType, SearchEngine, SearchItem};
use gonk_types::Song;

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
    fn update_engine(db: &Database) -> SearchEngine {
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
        let engine = Search::update_engine(db);

        Self {
            db,
            engine,
            query: String::new(),
            prev_query: String::new(),
            results: Index::default(),
            mode: SearchMode::Search,
        }
    }
    //TODO: this function name is misleading
    pub fn get_songs(&mut self) -> Option<Vec<Song>> {
        if let SearchMode::Search = self.mode {
            if !self.is_empty() {
                self.mode.next();
                self.results.select(Some(0));
            }
            None
        } else if let Some(item) = self.results.selected() {
            match item.item_type {
                ItemType::Song => Some(vec![self.db.get_song_from_id(item.song_id.unwrap())]),
                ItemType::Album => Some(
                    self.db
                        .album(item.album_artist.as_ref().unwrap(), &item.name),
                ),
                ItemType::Artist => Some(self.db.artist(&item.name)),
            }
        } else {
            None
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
        self.engine = Search::update_engine(self.db);
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
    pub fn is_empty(&self) -> bool {
        self.results.is_empty() && self.query.is_empty()
    }
    pub fn query_len(&self) -> u16 {
        self.query.len() as u16
    }
    pub fn get_query(&self) -> String {
        self.query.clone()
    }
    pub fn results(&self) -> &Vec<SearchItem> {
        &self.results.data
    }
    pub fn selected(&self) -> Option<usize> {
        self.results.index
    }
    pub fn on_escape(&mut self) -> bool {
        match self.mode {
            SearchMode::Search => {
                if let SearchMode::Search = self.mode {
                    self.query.clear();
                    true
                } else {
                    false
                }
            }
            SearchMode::Select => {
                self.mode.next();
                self.results.select(None);
                false
            }
        }
    }
}

use gonk_database::Colors;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

impl<'a> Search<'a> {
    pub fn old_draw<B: Backend>(&self, f: &mut Frame<B>, db: &Database, colors: &Colors) {
        let area = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Percentage(90)].as_ref())
            .split(area);

        let p = Paragraph::new(vec![Spans::from(self.get_query())])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left);

        let results = self.results();

        let items: Vec<_> = results
            .iter()
            .map(|r| match r.item_type {
                ItemType::Song => {
                    let song = db.get_song_from_id(r.song_id.unwrap());
                    Row::new(vec![
                        Cell::from(format!(" {}", song.name))
                            .style(Style::default().fg(colors.title)),
                        Cell::from(song.album.to_owned()).style(Style::default().fg(colors.album)),
                        Cell::from(song.artist).style(Style::default().fg(colors.artist)),
                    ])
                }
                ItemType::Album => Row::new(vec![
                    Cell::from(format!(" {} (album)", r.name))
                        .style(Style::default().fg(colors.title)),
                    Cell::from("").style(Style::default().fg(colors.album)),
                    Cell::from(r.album_artist.as_ref().unwrap().clone())
                        .style(Style::default().fg(colors.artist)),
                ]),
                ItemType::Artist => Row::new(vec![
                    Cell::from(format!(" {} (artist)", r.name))
                        .style(Style::default().fg(colors.title)),
                    Cell::from("").style(Style::default().fg(colors.album)),
                    Cell::from("").style(Style::default().fg(colors.artist)),
                ]),
            })
            .collect();

        let t = Table::new(items)
            .header(
                Row::new(vec![
                    Cell::from(" Name").style(Style::default().fg(colors.title)),
                    Cell::from("Album").style(Style::default().fg(colors.album)),
                    Cell::from("Artist").style(Style::default().fg(colors.artist)),
                ])
                .bottom_margin(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .widths(&[
                Constraint::Percentage(43),
                Constraint::Percentage(29),
                Constraint::Percentage(27),
            ])
            .highlight_symbol(">");

        let mut state = TableState::default();
        state.select(self.selected());

        f.render_widget(p, chunks[0]);
        f.render_stateful_widget(t, chunks[1], &mut state);

        //Move the cursor position when typing
        if let SearchMode::Search = self.mode {
            if self.results.is_none() && self.query.is_empty() {
                f.set_cursor(1, 1);
            } else {
                let mut len = self.query_len();
                if len > area.width {
                    len = area.width;
                }
                f.set_cursor(len + 1, 1);
            }
        }
    }
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

        if !self.results.is_empty() {
            if let Some(first) = self.results.data.first() {
                match first.item_type {
                    ItemType::Song => self.draw_song(f, h, first),
                    ItemType::Album => self.draw_album(f, h, first),
                    ItemType::Artist => self.draw_artist(f, h, &first.name),
                }
            }
            self.draw_other_results(f, v[2], colors);
        } else {
            self.draw_other_results(f, v[1].union(v[2]), colors);
        }

        self.handle_cursor(f);
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
    fn draw_other_results<B: Backend>(&self, f: &mut Frame<B>, area: Rect, colors: &Colors) {
        let results = self.results();

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

        let rows: Vec<_> = results
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if i == 0 {
                    get_cell(item, Modifier::ITALIC)
                } else {
                    get_cell(item, Modifier::empty())
                }
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
        state.select(self.selected());

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
    fn handle_cursor<B: Backend>(&self, f: &mut Frame<B>) {
        let area = f.size();
        //Move the cursor position when typing
        if let SearchMode::Search = self.mode {
            if self.results.is_none() && self.query.is_empty() {
                f.set_cursor(1, 1);
            } else {
                let mut len = self.query_len();
                if len > area.width {
                    len = area.width;
                }
                f.set_cursor(len + 1, 1);
            }
        }
    }
}

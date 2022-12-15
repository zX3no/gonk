use crate::{widgets::*, *};
use gonk_core::{vdb::Item, Song};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Search,
    Select,
}

pub struct Search {
    pub query: String,
    pub query_changed: bool,
    pub mode: Mode,
    pub results: Index<Item>,
}

impl Search {
    pub fn new() -> Self {
        let mut search = Self {
            query: String::new(),
            query_changed: false,
            mode: Mode::Search,
            results: Index::default(),
        };
        *search.results = unsafe { vdb::search(&VDB, &search.query) };
        search
    }
}

impl Widget for Search {
    fn up(&mut self) {
        self.results.up();
    }

    fn down(&mut self) {
        self.results.down();
    }

    fn left(&mut self) {}

    fn right(&mut self) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, mouse_event: Option<MouseEvent>) {
        let search = self;

        if search.query_changed {
            search.query_changed = !search.query_changed;
            *search.results = unsafe { vdb::search(&VDB, &search.query) };
        }

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(30),
                Constraint::Percentage(60),
            ])
            .split(area);

        if let Some(event) = mouse_event {
            let rect = Rect {
                x: event.column,
                y: event.row,
                ..Default::default()
            };
            if rect.intersects(v[0]) || rect.intersects(v[1]) {
                search.mode = Mode::Search;
                search.results.select(None);
            } else if rect.intersects(v[2]) && !search.results.is_empty() {
                search.mode = Mode::Select;
                search.results.select(Some(0));
            }
        }

        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(v[1]);

        draw_textbox(search, f, v[0]);

        let item = if search.results.selected().is_some() {
            search.results.selected()
        } else {
            search.results.first()
        };

        if let Some(item) = item {
            match item {
                Item::Song((artist, album, name, _, _)) => {
                    search::draw_song(f, name, album, artist, h[0]);
                    draw_album(f, album, artist, h[1]);
                }
                Item::Album((artist, album)) => {
                    search::draw_album(f, album, artist, h[0]);
                    draw_artist(f, artist, h[1]);
                }
                Item::Artist(artist) => {
                    let albums = unsafe { vdb::albums_by_artist(&VDB, artist).unwrap() };

                    search::draw_artist(f, artist, h[0]);

                    if albums.len() > 1 {
                        let h_split = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .split(h[1]);

                        //Draw the first two albums.
                        for (i, area) in h_split.iter().enumerate() {
                            if let Some(album) = albums.get(i) {
                                search::draw_album(f, &album.title, artist, *area);
                            }
                        }
                    } else if let Some(album) = albums.get(0) {
                        //Draw the first album.
                        search::draw_album(f, &album.title, artist, h[1]);
                    } else {
                        f.render_widget(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_type(BorderType::Rounded)
                                .title("Album"),
                            h[1],
                        );
                    };
                }
            }
            draw_results(search, f, v[2]);
        } else {
            draw_results(search, f, v[1].union(v[2]));
        }

        //Move the cursor position when typing
        if let Mode::Search = search.mode {
            if search.results.index().is_none() && search.query.is_empty() {
                f.set_cursor(1, 1);
            } else {
                let len = search.query.len() as u16;
                let max_width = area.width.saturating_sub(2);
                if len >= max_width {
                    f.set_cursor(max_width, 1);
                } else {
                    f.set_cursor(len + 1, 1);
                }
            }
        }
    }
}

pub fn on_backspace(search: &mut Search, control: bool) {
    match search.mode {
        Mode::Search if !search.query.is_empty() => {
            if control {
                search.query.clear();
            } else {
                search.query.pop();
            }

            search.query_changed = true;
        }
        Mode::Select => {
            search.results.select(None);
            search.mode = Mode::Search;
        }
        _ => (),
    }
}

pub fn on_enter(search: &mut Search) -> Option<Vec<&'static Song>> {
    match search.mode {
        Mode::Search => {
            if !search.results.is_empty() {
                search.mode = Mode::Select;
                search.results.select(Some(0));
            }
            None
        }
        Mode::Select => search.results.selected().map(|item| match item {
            Item::Song((artist, album, _, disc, number)) => {
                match unsafe { vdb::song(&VDB, artist, album, *disc, *number) } {
                    Some(song) => vec![song],
                    None => panic!("{item:?}"),
                }
            }
            Item::Album((artist, album)) => unsafe { vdb::album(&VDB, artist, album) }
                .unwrap()
                .songs
                .iter()
                .collect(),
            Item::Artist(artist) => {
                let artist = unsafe { vdb::artist(&VDB, artist).unwrap() };
                let mut songs = Vec::new();
                for album in artist {
                    for song in &album.songs {
                        songs.push(song);
                    }
                }
                songs
            }
        }),
    }
}

fn draw_song(f: &mut Frame, name: &str, album: &str, artist: &str, area: Rect) {
    let rows = [
        Row::new(vec![Spans::from(Span::raw(album))]),
        Row::new(vec![Spans::from(Span::raw(artist))]),
    ];
    let song_table = Table::new(&rows)
        .header(
            Row::new(vec![Span::styled(
                format!("{name} "),
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

fn draw_album(f: &mut Frame, album: &str, artist: &str, area: Rect) {
    let cells: Vec<Row> = unsafe { vdb::album(&VDB, artist, album).unwrap() }
        .songs
        .iter()
        .map(|song| {
            Row::new(vec![Cell::from(format!(
                "{}. {}",
                song.track_number, song.title
            ))])
        })
        .collect();

    let table = Table::new(&cells)
        .header(
            Row::new(vec![Cell::from(Span::styled(
                format!("{album} "),
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

    f.render_widget(table, area);
}

fn draw_artist(f: &mut Frame, artist: &str, area: Rect) {
    let albums = unsafe { vdb::albums_by_artist(&VDB, artist).unwrap() };
    let cells: Vec<_> = albums
        .iter()
        .map(|album| Row::new(vec![Cell::from(Span::raw(&album.title))]))
        .collect();

    let table = Table::new(&cells)
        .header(
            Row::new(vec![Cell::from(Span::styled(
                format!("{artist} "),
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

    f.render_widget(table, area);
}

fn draw_results<'a>(search: &'a Search, f: &mut Frame, area: Rect) {
    let get_cell = |item: &'a Item, selected: bool| -> Row {
        let selected_cell = if selected {
            Cell::from(">")
        } else {
            Cell::default()
        };

        match item {
            Item::Song((artist, album, name, _, _)) => Row::new(vec![
                selected_cell,
                Cell::from(name.as_str()).style(Style::default().fg(TITLE)),
                Cell::from(album.as_str()).style(Style::default().fg(ALBUM)),
                Cell::from(artist.as_str()).style(Style::default().fg(ARTIST)),
            ]),
            Item::Album((artist, album)) => Row::new(vec![
                selected_cell,
                Cell::from(Spans::from(vec![
                    Span::styled(format!("{album} - "), Style::default().fg(TITLE)),
                    Span::styled(
                        "Album",
                        Style::default().fg(TITLE).add_modifier(Modifier::ITALIC),
                    ),
                ])),
                Cell::from("").style(Style::default().fg(ALBUM)),
                Cell::from(artist.as_str()).style(Style::default().fg(ARTIST)),
            ]),
            Item::Artist(artist) => Row::new(vec![
                selected_cell,
                Cell::from(Spans::from(vec![
                    Span::styled(format!("{artist} - "), Style::default().fg(TITLE)),
                    Span::styled(
                        "Artist",
                        Style::default().fg(TITLE).add_modifier(Modifier::ITALIC),
                    ),
                ])),
                Cell::from("").style(Style::default().fg(ALBUM)),
                Cell::from("").style(Style::default().fg(ARTIST)),
            ]),
        }
    };

    let rows: Vec<Row> = search
        .results
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if let Some(s) = search.results.index() {
                if s == i {
                    return get_cell(item, true);
                }
            } else if i == 0 {
                return get_cell(item, false);
            }
            get_cell(item, false)
        })
        .collect();

    let italic = Style::default().add_modifier(Modifier::ITALIC);
    let table = Table::new(&rows)
        .header(
            Row::new(vec![
                Cell::default(),
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
            Constraint::Length(1),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
            Constraint::Percentage(20),
        ]);

    f.render_stateful_widget(table, area, &mut TableState::new(search.results.index()));
}

fn draw_textbox(search: &Search, f: &mut Frame, area: Rect) {
    let len = search.query.len() as u16;
    //Search box is a little smaller than the max width
    let width = area.width.saturating_sub(1);
    let offset_x = if len < width { 0 } else { len - width + 1 };

    f.render_widget(
        Paragraph::new(search.query.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left)
            .scroll((0, offset_x)),
        area,
    );
}

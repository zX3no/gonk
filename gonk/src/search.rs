use super::Mode as AppMode;
use crate::widgets::*;
use crate::*;
use crossterm::event::KeyModifiers;
use gonk_player::{Index, Player};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::cmp::Ordering;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Clone)]
pub enum Item {
    Song(Song),
    Album(Album),
    Artist(Artist),
}

#[derive(Clone, Default)]
pub struct Song {
    pub id: usize,
    pub name: String,
    pub album: String,
    pub artist: String,
}

#[derive(Clone, Default)]
pub struct Album {
    pub name: String,
    pub artist: String,
}

#[derive(Clone, Default)]
pub struct Artist {
    pub name: String,
}

#[derive(PartialEq, Eq)]
pub enum Mode {
    Search,
    Select,
}

pub struct Search {
    query: String,
    query_changed: bool,
    mode: Mode,
    results: Index<Item>,
    cache: Vec<Item>,
}

impl Search {
    pub fn new() -> Self {
        let mut search = Self {
            cache: Vec::new(),
            query: String::new(),
            query_changed: false,
            mode: Mode::Search,
            results: Index::default(),
        };
        update(&mut search);
        search
    }
}

pub fn get_input(search: &Search) -> bool {
    search.mode == Mode::Search
}

pub fn update(search: &mut Search) {
    update_cache(search);
    update_search(search);
}

fn update_cache(search: &mut Search) {
    search.cache = Vec::new();

    for song in sqlite::get_all_songs() {
        search.cache.push(Item::Song(Song {
            name: song.name,
            album: song.album,
            artist: song.artist,
            id: song.id.unwrap(),
        }));
    }

    for (name, artist) in sqlite::get_all_albums() {
        search.cache.push(Item::Album(Album { name, artist }));
    }

    for name in sqlite::get_all_artists() {
        search.cache.push(Item::Artist(Artist { name }));
    }
}
fn update_search(search: &mut Search) {
    let query = &search.query.to_lowercase();

    let mut results: Vec<_> = if query.is_empty() {
        //If there user has not asked to search anything
        //populate the list with 40 results.
        search
            .cache
            .iter()
            .take(40)
            .rev()
            .map(|item| {
                let acc = match item {
                    Item::Song(song) => strsim::jaro_winkler(query, &song.name.to_lowercase()),
                    Item::Album(album) => strsim::jaro_winkler(query, &album.name.to_lowercase()),
                    Item::Artist(artist) => {
                        strsim::jaro_winkler(query, &artist.name.to_lowercase())
                    }
                };

                (item, acc)
            })
            .collect()
    } else {
        search
            .cache
            .par_iter()
            .filter_map(|item| {
                //I don't know if 'to_lowercase' has any overhead.
                let acc = match item {
                    Item::Song(song) => strsim::jaro_winkler(query, &song.name.to_lowercase()),
                    Item::Album(album) => strsim::jaro_winkler(query, &album.name.to_lowercase()),
                    Item::Artist(artist) => {
                        strsim::jaro_winkler(query, &artist.name.to_lowercase())
                    }
                };

                //Filter out results that are poor matches. 0.75 is an arbitrary value.
                if acc > 0.75 {
                    Some((item, acc))
                } else {
                    None
                }
            })
            .collect()
    };

    //Sort results by score.
    results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

    //Sort artists above search-titled albums.
    results.sort_by(|(item, a), (_, b)| {
        //If the score is the same
        if a == b {
            //And the item is an album
            if let Item::Album(_) = item {
                //Move item lower in the list.
                Ordering::Greater
            } else {
                //Move item higher in the list.
                Ordering::Less
            }
        } else {
            //Keep the same order.
            Ordering::Equal
        }
    });

    search.results.data = results.into_iter().map(|(item, _)| item.clone()).collect();
}
impl Input for Search {
    fn up(&mut self) {
        self.results.up();
    }

    fn down(&mut self) {
        self.results.down();
    }

    fn left(&mut self) {
        todo!()
    }

    fn right(&mut self) {
        todo!()
    }
}

pub fn on_key(search: &mut Search, c: char) {
    if search.mode == Mode::Search {
        search.query_changed = true;
        search.query.push(c);
    }
}

pub fn on_backspace(search: &mut Search, shift: bool) {
    match search.mode {
        Mode::Search => {
            if shift {
                search.query.clear();
            } else {
                search.query.pop();
            }
        }
        Mode::Select => {
            search.results.select(None);
            search.mode = Mode::Search;
        }
    }
}

pub fn on_escape(search: &mut Search, mode: &mut AppMode) {
    match search.mode {
        Mode::Search => {
            if let Mode::Search = search.mode {
                search.query.clear();
                *mode = AppMode::Queue;
            }
        }
        Mode::Select => {
            search.mode = Mode::Search;
            search.results.select(None);
        }
    }
}

pub fn on_enter(search: &mut Search, player: &mut Player) {
    match search.mode {
        Mode::Search => {
            if !search.results.is_empty() {
                search.mode = Mode::Select;
                search.results.select(Some(0));
            }
        }
        Mode::Select => {
            if let Some(item) = search.results.selected() {
                let songs = match item {
                    Item::Song(song) => sqlite::get_songs(&[song.id]),
                    Item::Album(album) => {
                        sqlite::get_all_songs_from_album(&album.name, &album.artist)
                    }
                    Item::Artist(artist) => sqlite::get_songs_by_artist(&artist.name),
                };

                player.add_songs(&songs);
            }
        }
    }
}

pub fn draw(search: &mut Search, area: Rect, f: &mut Frame) {
    if search.query_changed {
        update_search(search);
    }

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(30),
            Constraint::Percentage(60),
        ])
        .split(area);

    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(v[1]);

    draw_textbox(search, f, v[0]);

    let item = if search.results.selected().is_some() {
        search.results.selected()
    } else {
        search.results.data.first()
    };

    if let Some(item) = item {
        match item {
            Item::Song(song) => {
                search::song(f, &song.name, &song.album, &song.artist, h[0]);
                album(search, f, &song.album, &song.artist, h[1]);
            }
            Item::Album(album) => {
                search::album(search, f, &album.name, &album.artist, h[0]);
                artist(search, f, &album.artist, h[1]);
            }
            Item::Artist(artist) => {
                let albums = sqlite::get_all_albums_by_artist(&artist.name);

                search::artist(search, f, &artist.name, h[0]);

                let h_split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(h[1]);

                //draw the first two albums
                for (i, area) in h_split.iter().enumerate() {
                    if let Some(album) = albums.get(i) {
                        search::album(search, f, album, &artist.name, *area);
                    }
                }
            }
        }
        draw_results(search, f, v[2]);
    } else {
        draw_results(search, f, v[1].union(v[2]));
    }

    update_cursor(search, f);
}

fn song(f: &mut Frame, name: &str, album: &str, artist: &str, area: Rect) {
    let song_table = Table::new(vec![
        Row::new(vec![Spans::from(Span::raw(album))]),
        Row::new(vec![Spans::from(Span::raw(artist))]),
    ])
    .header(
        Row::new(vec![Span::styled(
            format!("{} ", name),
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

fn album(search: &Search, f: &mut Frame, album: &str, artist: &str, area: Rect) {
    let cells: Vec<_> = sqlite::get_all_songs_from_album(album, artist)
        .iter()
        .map(|song| Row::new(vec![Cell::from(format!("{}. {}", song.number, song.name))]))
        .collect();

    let table = Table::new(cells)
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

    f.render_widget(table, area);
}

fn artist(search: &Search, f: &mut Frame, artist: &str, area: Rect) {
    let albums = sqlite::get_all_albums_by_artist(artist);
    let cells: Vec<_> = albums
        .iter()
        .map(|album| Row::new(vec![Cell::from(Span::raw(album))]))
        .collect();

    let table = Table::new(cells)
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

    f.render_widget(table, area);
}

fn draw_results(search: &Search, f: &mut Frame, area: Rect) {
    let get_cell = |item: &Item, selected: bool| -> Row {
        let selected_cell = if selected {
            Cell::from(">")
        } else {
            Cell::default()
        };

        match item {
            Item::Song(song) => {
                let song = sqlite::get_songs(&[song.id])[0].clone();
                Row::new(vec![
                    selected_cell,
                    Cell::from(song.name).style(Style::default().fg(COLORS.name)),
                    Cell::from(song.album).style(Style::default().fg(COLORS.album)),
                    Cell::from(song.artist).style(Style::default().fg(COLORS.artist)),
                ])
            }
            Item::Album(album) => Row::new(vec![
                selected_cell,
                Cell::from(format!("{} - Album", album.name))
                    .style(Style::default().fg(COLORS.name)),
                Cell::from("").style(Style::default().fg(COLORS.album)),
                Cell::from(album.artist.clone()).style(Style::default().fg(COLORS.artist)),
            ]),
            Item::Artist(artist) => Row::new(vec![
                selected_cell,
                Cell::from(format!("{} - Artist", artist.name))
                    .style(Style::default().fg(COLORS.name)),
                Cell::from("").style(Style::default().fg(COLORS.album)),
                Cell::from("").style(Style::default().fg(COLORS.artist)),
            ]),
        }
    };

    let rows: Vec<_> = search
        .results
        .data
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
    let table = Table::new(rows)
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

fn update_cursor(search: &Search, f: &mut Frame) {
    let area = f.size();
    //Move the cursor position when typing
    if let Mode::Search = search.mode {
        if search.results.is_none() && search.query.is_empty() {
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

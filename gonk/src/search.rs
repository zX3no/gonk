use crate::widgets::*;
use crate::*;
use gonk_database::Song;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::cmp::Ordering;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

const MIN_ACCURACY: f64 = 0.70;

#[derive(Clone, Debug)]
pub enum Item {
    Song(MinSong),
    Album(Album),
    Artist(Artist),
}

#[derive(Clone, Default, Debug)]
pub struct MinSong {
    pub id: usize,
    pub name: String,
    pub album: String,
    pub artist: String,
}

#[derive(Clone, Default, Debug)]
pub struct Album {
    pub name: String,
    pub artist: String,
}

#[derive(Clone, Default, Debug)]
pub struct Artist {
    pub name: String,
}

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
    pub cache: Vec<Item>,
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
        refresh_cache(&mut search);
        refresh_results(&mut search);
        search
    }
}

impl Input for Search {
    fn up(&mut self) {
        self.results.up();
    }

    fn down(&mut self) {
        self.results.down();
    }

    fn left(&mut self) {}

    fn right(&mut self) {}
}

pub fn on_backspace(search: &mut Search, shift: bool) {
    match search.mode {
        Mode::Search => {
            if shift {
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
    }
}

pub fn on_escape(search: &mut Search) {
    match search.mode {
        Mode::Search => {
            if let Mode::Search = search.mode {
                search.query.clear();
                search.query_changed = true;
            }
        }
        Mode::Select => {
            search.mode = Mode::Search;
            search.results.select(None);
        }
    }
}

pub fn on_enter(search: &mut Search) -> Option<Vec<Song>> {
    match search.mode {
        Mode::Search => {
            if !search.results.is_empty() {
                search.mode = Mode::Select;
                search.results.select(Some(0));
            }
            None
        }
        Mode::Select => search.results.selected().map(|item| match item {
            Item::Song(song) => gonk_database::ids(&[song.id]),
            Item::Album(album) => gonk_database::songs_from_album(&album.artist, &album.name),
            Item::Artist(artist) => gonk_database::songs_by_artist(&artist.name),
        }),
    }
}

pub fn refresh_cache(search: &mut Search) {
    search.cache = Vec::new();

    let (artists, albums, songs) = gonk_database::artists_albums_and_songs();

    for song in songs {
        search.cache.push(Item::Song(MinSong {
            name: song.title,
            album: song.album,
            artist: song.artist,
            id: song.id,
        }));
    }

    for (artist, album) in albums {
        search.cache.push(Item::Album(Album {
            name: album,
            artist,
        }));
    }

    for artist in artists {
        search.cache.push(Item::Artist(Artist { name: artist }));
    }
}

pub fn get_item_accuracy(query: &str, item: &Item) -> f64 {
    match item {
        Item::Song(song) => strsim::jaro_winkler(query, &song.name.to_lowercase()),
        Item::Album(album) => strsim::jaro_winkler(query, &album.name.to_lowercase()),
        Item::Artist(artist) => strsim::jaro_winkler(query, &artist.name.to_lowercase()),
    }
}

pub fn refresh_results(search: &mut Search) {
    if search.query.is_empty() {
        //If there user has not asked to search anything
        //populate the list with 40 results.
        search.results.data = search.cache.iter().take(40).cloned().collect();
        return;
    }

    let query = &search.query.to_lowercase();

    //Collect all results that are close to the search query.
    let mut results: Vec<(&Item, f64)> = search
        .cache
        .par_iter()
        .filter_map(|item| {
            let acc = get_item_accuracy(query, item);
            if acc > MIN_ACCURACY {
                Some((item, acc))
            } else {
                None
            }
        })
        .collect();

    //Sort results by score.
    results.sort_unstable_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

    if results.len() > 25 {
        //Remove the less accurate results.
        results.drain(25..);
    }

    //Sort songs with equal score. Artist > Album > Song.
    results.sort_unstable_by(|(item_1, score_1), (item_2, score_2)| {
        if score_1 == score_2 {
            match item_1 {
                Item::Artist(_) => match item_2 {
                    Item::Song(_) => Ordering::Less,
                    Item::Album(_) => Ordering::Less,
                    Item::Artist(_) => Ordering::Equal,
                },
                Item::Album(_) => match item_2 {
                    Item::Song(_) => Ordering::Less,
                    Item::Album(_) => Ordering::Equal,
                    Item::Artist(_) => Ordering::Greater,
                },
                Item::Song(_) => match item_2 {
                    Item::Song(_) => Ordering::Equal,
                    Item::Album(_) => Ordering::Greater,
                    Item::Artist(_) => Ordering::Greater,
                },
            }
        } else if score_2 > score_1 {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    });

    search.results.data = results.into_iter().map(|(item, _)| item.clone()).collect();

    //TODO: Tell the user how long the search took.
    // println!(" {:?}", now.elapsed());
}

pub fn draw(search: &mut Search, area: Rect, f: &mut Frame) {
    if search.query_changed {
        search.query_changed = !search.query_changed;
        refresh_results(search);
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
                search::draw_song(f, &song.name, &song.album, &song.artist, h[0]);
                draw_album(f, &song.album, &song.artist, h[1]);
            }
            Item::Album(album) => {
                search::draw_album(f, &album.name, &album.artist, h[0]);
                draw_artist(f, &album.artist, h[1]);
            }
            Item::Artist(artist) => {
                let albums = gonk_database::albums_by_artist(&artist.name);

                search::draw_artist(f, &artist.name, h[0]);

                if albums.len() > 1 {
                    let h_split = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(h[1]);

                    //Draw the first two albums.
                    for (i, area) in h_split.iter().enumerate() {
                        if let Some(album) = albums.get(i) {
                            search::draw_album(f, album, &artist.name, *area);
                        }
                    }
                } else if let Some(album) = albums.get(0) {
                    //Draw the first album.
                    search::draw_album(f, album, &artist.name, h[1]);
                } else {
                    f.render_widget(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title("Album"),
                        h[1],
                    )
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

fn draw_song(f: &mut Frame, name: &str, album: &str, artist: &str, area: Rect) {
    let rows = [
        Row::new(vec![Spans::from(Span::raw(album))]),
        Row::new(vec![Spans::from(Span::raw(artist))]),
    ];
    let song_table = Table::new(&rows)
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

fn draw_album(f: &mut Frame, album: &str, artist: &str, area: Rect) {
    let cells: Vec<Row> = gonk_database::songs_from_album(artist, album)
        .iter()
        .map(|song| Row::new(vec![Cell::from(format!("{}. {}", song.number, song.title))]))
        .collect();

    let table = Table::new(&cells)
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

fn draw_artist(f: &mut Frame, artist: &str, area: Rect) {
    let albums = gonk_database::albums_by_artist(artist);
    let cells: Vec<_> = albums
        .iter()
        .map(|album| Row::new(vec![Cell::from(Span::raw(album))]))
        .collect();

    let table = Table::new(&cells)
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
                let song = gonk_database::get(song.id).unwrap();
                Row::new(vec![
                    selected_cell,
                    Cell::from(song.title).style(Style::default().fg(COLORS.title)),
                    Cell::from(song.album).style(Style::default().fg(COLORS.album)),
                    Cell::from(song.artist).style(Style::default().fg(COLORS.artist)),
                ])
            }
            Item::Album(album) => Row::new(vec![
                selected_cell,
                Cell::from(Spans::from(vec![
                    Span::styled(
                        format!("{} - ", album.name),
                        Style::default().fg(COLORS.title),
                    ),
                    Span::styled(
                        "Album",
                        Style::default()
                            .fg(COLORS.title)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])),
                Cell::from("").style(Style::default().fg(COLORS.album)),
                Cell::from(album.artist.clone()).style(Style::default().fg(COLORS.artist)),
            ]),
            Item::Artist(artist) => Row::new(vec![
                selected_cell,
                Cell::from(Spans::from(vec![
                    Span::styled(
                        format!("{} - ", artist.name),
                        Style::default().fg(COLORS.title),
                    ),
                    Span::styled(
                        "Artist",
                        Style::default()
                            .fg(COLORS.title)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])),
                Cell::from("").style(Style::default().fg(COLORS.album)),
                Cell::from("").style(Style::default().fg(COLORS.artist)),
            ]),
        }
    };

    let rows: Vec<Row> = search
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

mod strsim {
    use std::cmp::{max, min};

    pub fn jaro_winkler(a: &str, b: &str) -> f64 {
        let jaro_distance = generic_jaro(a, b);

        // Don't limit the length of the common prefix
        let prefix_length = a
            .chars()
            .zip(b.chars())
            .take_while(|&(ref a_elem, ref b_elem)| a_elem == b_elem)
            .count();

        let jaro_winkler_distance =
            jaro_distance + (0.1 * prefix_length as f64 * (1.0 - jaro_distance));

        if jaro_winkler_distance <= 1.0 {
            jaro_winkler_distance
        } else {
            1.0
        }
    }

    pub fn generic_jaro(a: &str, b: &str) -> f64 {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        // The check for lengths of one here is to prevent integer overflow when
        // calculating the search range.
        if a_len == 0 && b_len == 0 {
            return 1.0;
        } else if a_len == 0 || b_len == 0 {
            return 0.0;
        } else if a_len == 1 && b_len == 1 {
            return if a.chars().eq(b.chars()) { 1.0 } else { 0.0 };
        }

        let search_range = (max(a_len, b_len) / 2) - 1;

        let mut b_consumed = vec![false; b_len];
        let mut matches = 0.0;

        let mut transpositions = 0.0;
        let mut b_match_index = 0;

        for (i, a_elem) in a.chars().enumerate() {
            let min_bound =
            // prevent integer wrapping
            if i > search_range {
                max(0, i - search_range)
            } else {
                0
            };

            let max_bound = min(b_len - 1, i + search_range);

            if min_bound > max_bound {
                continue;
            }

            for (j, b_elem) in b.chars().enumerate() {
                if min_bound <= j && j <= max_bound && a_elem == b_elem && !b_consumed[j] {
                    b_consumed[j] = true;
                    matches += 1.0;

                    if j < b_match_index {
                        transpositions += 1.0;
                    }
                    b_match_index = j;

                    break;
                }
            }
        }

        if matches == 0.0 {
            0.0
        } else {
            (1.0 / 3.0)
                * ((matches / a_len as f64)
                    + (matches / b_len as f64)
                    + ((matches - transpositions) / matches))
        }
    }
}

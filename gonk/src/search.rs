use crate::{
    widgets::{Cell, Row, Table, TableState},
    Frame, ALBUM, ARTIST, SEARCH_MARGIN, TITLE,
};
use crossterm::event::MouseEvent;
use gonk_core::{
    vdb::{Database, Item},
    Index, Song,
};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Search,
    Select,
}

pub struct Search<'a> {
    pub query: String,
    pub query_changed: bool,
    pub mode: Mode,
    pub results: Index<Item<'a>>,
}

impl<'a> Search<'a> {
    pub fn new(db: &'a Database) -> Self {
        let mut search = Self {
            query: String::new(),
            query_changed: false,
            mode: Mode::Search,
            results: Index::default(),
        };
        *search.results = db.search(&search.query);
        search
    }
}

pub fn up(search: &mut Search) {
    search.results.up();
}

pub fn down(search: &mut Search) {
    search.results.down();
}

//TODO: Artist and albums colors aren't quite right.
pub fn draw<'a>(
    search: &'a mut Search<'a>,
    f: &mut Frame,
    area: Rect,
    mouse_event: Option<MouseEvent>,
    db: &'a Database,
) {
    let area = area.inner(&SEARCH_MARGIN);
    f.render_widget(Clear, area);

    if search.query_changed {
        search.query_changed = !search.query_changed;
        *search.results = db.search(&search.query);
    }

    let layout_margin = 1;
    let v = Layout::default()
        .margin(layout_margin)
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(40)])
        .split(area);

    if let Some(event) = mouse_event {
        let rect = Rect {
            x: event.column,
            y: event.row,
            ..Default::default()
        };
        if rect.intersects(v[0]) {
            search.mode = Mode::Search;
            search.results.select(None);
        } else if rect.intersects(v[1]) && !search.results.is_empty() {
            search.mode = Mode::Select;
            search.results.select(Some(0));
        }
    }

    let len = search.query.len() as u16;
    //Search box is a little smaller than the max width
    let width = area.width.saturating_sub(1);
    let offset_x = if len < width { 0 } else { len - width + 1 };

    f.render_widget(
        Paragraph::new(search.query.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Search:"),
            )
            .alignment(Alignment::Left)
            .scroll((0, offset_x)),
        v[0],
    );

    let get_cell = |item: &'a Item, selected: bool| -> Row {
        let selected_cell = if selected {
            Cell::from(">")
        } else {
            Cell::default()
        };

        match item {
            Item::Song((artist, album, name, _, _)) => Row::new(vec![
                selected_cell,
                Cell::from(*name).style(Style::default().fg(TITLE)),
                Cell::from(*album).style(Style::default().fg(ALBUM)),
                Cell::from(*artist).style(Style::default().fg(ARTIST)),
            ]),
            Item::Album((artist, album)) => Row::new(vec![
                selected_cell,
                Cell::from(Spans::from(vec![
                    Span::styled(format!("{album} - "), Style::default().fg(ALBUM)),
                    Span::styled(
                        "Album",
                        Style::default().fg(ALBUM).add_modifier(Modifier::ITALIC),
                    ),
                ])),
                Cell::from("-"),
                Cell::from(*artist).style(Style::default().fg(ARTIST)),
            ]),
            Item::Artist(artist) => Row::new(vec![
                selected_cell,
                Cell::from(Spans::from(vec![
                    Span::styled(format!("{artist} - "), Style::default().fg(ARTIST)),
                    Span::styled(
                        "Artist",
                        Style::default().fg(ARTIST).add_modifier(Modifier::ITALIC),
                    ),
                ])),
                Cell::from("-"),
                Cell::from("-"),
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
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ]);

    f.render_stateful_widget(table, v[1], &mut TableState::new(search.results.index()));

    let y = SEARCH_MARGIN.vertical + 1 + layout_margin;
    let x = SEARCH_MARGIN.horizontal + 1 + layout_margin;

    //Move the cursor position when typing
    if let Mode::Search = search.mode {
        if search.results.index().is_none() && search.query.is_empty() {
            f.set_cursor(x, y);
        } else {
            let len = search.query.len() as u16;
            let max_width = area.width.saturating_sub(3);
            if len >= max_width {
                f.set_cursor(x - 1 + max_width, y);
            } else {
                f.set_cursor(x + len, y);
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

pub fn on_enter(search: &mut Search, db: &Database) -> Option<Vec<Song>> {
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
                vec![db.song(artist, album, *disc, *number).clone()]
            }
            Item::Album((artist, album)) => db.album(artist, album).songs.clone(),
            Item::Artist(artist) => db
                .albums_by_artist(artist)
                .iter()
                .flat_map(|album| album.songs.clone())
                .collect(),
        }),
    }
}

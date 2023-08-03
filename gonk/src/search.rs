use crate::{ALBUM, ARTIST, SEARCH_MARGIN, TITLE};
use gonk_core::{
    vdb::{Database, Item},
    Index, Song,
};
use winter::*;

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
        Self {
            query: String::new(),
            query_changed: false,
            mode: Mode::Search,
            results: Index::default(),
        }
    }
}

pub fn up(search: &mut Search) {
    search.results.up();
}

pub fn down(search: &mut Search) {
    search.results.down();
}

//TODO: Artist and albums colors aren't quite right.
pub fn draw(
    search: &mut Search,
    area: winter::Rect,
    buf: &mut winter::Buffer,
    mouse: Option<(u16, u16)>,
    db: &Database,
) {
    // let area = area.inner(SEARCH_MARGIN);
    //TODO: Clear
    // f.render_widget(Clear, area);

    if search.query_changed {
        search.query_changed = !search.query_changed;
        *search.results = db.search(&search.query);
    }

    let layout_margin = 1;
    let v = layout(area, Vertical, (1, 1), [Length(3), Min(40)].into());

    if let Some((x, y)) = mouse {
        let rect = Rect {
            x,
            y,
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

    //TODO: Scroll
    lines!(search.query.as_str())
        .block(Some("Search:".into()), ALL, Rounded)
        .draw(v[0], buf);
    // .scroll((0, offset_x)),

    let rows: Vec<Row> = search
        .results
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let Some(s) = search.results.index() else {
                return cell(item, false);
            };
            if s == i {
                cell(item, true)
            } else {
                cell(item, false)
            }
        })
        .collect();

    // let italic = Style::default().add_modifier(Modifier::ITALIC);
    let header = header![
        text!(),
        text!("Name", italic()),
        text!("Album", italic()),
        text!("Artist", italic())
    ];
    let table = table(
        Some(header),
        Some(block(None, ALL, Rounded)),
        &[
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ],
        rows,
        None,
        style(),
    );

    table.draw(v[1], buf, search.results.index());

    let x = SEARCH_MARGIN.0 + 1 + layout_margin;
    let y = SEARCH_MARGIN.1 + 1 + layout_margin;

    //TODO: Set cursor position.
    //Move the cursor position when typing
    if let Mode::Search = search.mode {
        if search.results.index().is_none() && search.query.is_empty() {
            // f.set_cursor(x, y);
        } else {
            let len = search.query.len() as u16;
            let max_width = area.width.saturating_sub(3);
            if len >= max_width {
                // f.set_cursor(x - 1 + max_width, y);
            } else {
                // f.set_cursor(x + len, y);
            }
        }
    }
}

fn cell<'a>(item: &'a Item, selected: bool) -> Row<'a> {
    let selected_cell = if selected { ">" } else { "" };

    match item {
        Item::Song((artist, album, name, _, _)) => row![
            text!(selected_cell),
            text!(name.as_str(), fg(TITLE)),
            text!(album.as_str(), fg(ALBUM)),
            text!(artist.as_str(), fg(ARTIST))
        ],
        Item::Album((artist, album)) => row![
            text!(selected_cell),
            lines_s!(
                format!("{album} - "),
                fg(ALBUM),
                "Album",
                fg(ALBUM).italic()
            ),
            text!("-"),
            text!(artist.as_str(), fg(ARTIST))
        ],
        Item::Artist(artist) => row![
            text!(selected_cell),
            lines_s!(
                format!("{artist} - "),
                fg(ARTIST),
                "Artist",
                fg(ARTIST).italic()
            ),
            text!("-"),
            text!("-")
        ],
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

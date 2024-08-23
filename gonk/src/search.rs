use crate::{ALBUM, ARTIST, TITLE};
use gonk_core::{
    vdb::{Database, Item},
    Index, Song,
};
use winter::*;

#[derive(PartialEq, Eq, Debug)]
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

//TODO: Artist and albums colors aren't quite right.
pub fn draw(
    search: &mut Search,
    area: winter::Rect,
    buf: &mut winter::Buffer,
    mouse: Option<(u16, u16)>,
    db: &Database,
) -> Option<(u16, u16)> {
    if search.query_changed {
        search.query_changed = !search.query_changed;
        *search.results = db.search(&search.query);
    }

    let v = layout(area, Vertical, &[Length(3), Fill]);

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

    lines!(search.query.as_str())
        .block(block().title("Search:"))
        .scroll()
        .draw(v[0], buf);

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

    let table = table(
        rows,
        &[
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ],
    )
    .header(header![
        text!(),
        "Name".italic(),
        "Album".italic(),
        "Artist".italic()
    ])
    .block(block());

    table.draw(v[1], buf, search.results.index());

    let layout_margin = 1;
    let x = 1 + layout_margin;
    let y = 1 + layout_margin;

    if let Mode::Search = search.mode {
        if search.results.index().is_none() && search.query.is_empty() {
            Some((x, y))
        } else {
            let len = search.query.len() as u16;
            let max_width = area.width.saturating_sub(3);
            if len >= max_width {
                Some((x - 1 + max_width, y))
            } else {
                Some((x + len, y))
            }
        }
    } else {
        None
    }
}

//Items have a lifetime of 'search because they live in the Search struct.
fn cell(item: &Item, selected: bool) -> Row<'_> {
    let selected_cell = if selected { ">" } else { "" };

    match item {
        Item::Song((artist, album, name, _, _)) => row![
            selected_cell,
            name.as_str().fg(TITLE),
            album.as_str().fg(ALBUM),
            artist.as_str().fg(ARTIST)
        ],
        Item::Album((artist, album)) => row![
            selected_cell,
            lines!(text!("{album} - ").fg(ALBUM), "Album".fg(ALBUM).italic()),
            "-",
            artist.fg(ARTIST)
        ],
        Item::Artist(artist) => row![
            selected_cell,
            lines!(
                text!("{artist} - ").fg(ARTIST),
                "Artist".fg(ARTIST).italic()
            ),
            "-",
            "-"
        ],
    }
}

pub fn on_backspace(search: &mut Search, control: bool, shift: bool) {
    match search.mode {
        Mode::Search if !search.query.is_empty() => {
            if shift && control {
                search.query.clear();
            } else if control {
                let trim = search.query.trim_end();
                let end = trim.chars().rev().position(|c| c == ' ');
                if let Some(end) = end {
                    search.query = trim[..trim.len() - end].to_string();
                } else {
                    search.query.clear();
                }
            } else {
                search.query.pop();
            }

            search.query_changed = true;
        }
        Mode::Search => {}
        Mode::Select => {
            search.results.select(None);
            search.mode = Mode::Search;
        }
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

use crate::theme::colors;
use mu_core::vdb::{Database, Item};
use mu_core::{Index, Song};
use neoui::*;
use std::time::Instant;

pub struct Search {
    pub query: String,
    pub focused: bool,
    pub results: Index<Item>,
    pub dirty: bool,
    pub backspace_held_since: Option<Instant>,
    pub backspace_last_tick: Option<Instant>,
}

impl Search {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            focused: false,
            results: Index::default(),
            dirty: false,
            backspace_held_since: None,
            backspace_last_tick: None,
        }
    }
}

pub enum Action {
    OpenArtist(String),
    Play(Song),
    Append(Song),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    search: &mut Search,
    db: &Database,
    artists: &[String],
    scroll: &mut usize,
) -> Option<Action> {
    let (head, body) = ui.split_rect_v(rect, 90);
    ui.paint_rect(head, style().bg(colors::BG));
    ui.paint_text(
        "Search",
        head.x + 40,
        head.y + 20,
        200,
        28,
        colors::TEXT,
        0,
        28,
        Alignment::Left,
        Padding::default(),
        0,
    );

    let box_rect = Rect::new(head.x + 40, head.y + 54, (head.width - 80).min(480), 28);
    ui.paint_rect(
        box_rect,
        style()
            .bg(colors::PANEL_RAISED)
            .border(if search.focused {
                colors::ACCENT
            } else {
                colors::LINE
            })
            .radius(8),
    );
    let display = if search.query.is_empty() {
        "Search artists, albums, songs…".to_string()
    } else {
        search.query.clone()
    };
    ui.paint_text(
        display,
        box_rect.x + 12,
        box_rect.y,
        box_rect.width - 24,
        box_rect.height,
        if search.query.is_empty() {
            colors::TEXT_DIM
        } else {
            colors::TEXT
        },
        0,
        14,
        Alignment::Left,
        Padding::default(),
        0,
    );
    if ui.clicked(box_rect) {
        search.focused = true;
    }

    if search.dirty {
        let q = search.query.clone();
        search.results = if q.is_empty() {
            Index::default()
        } else {
            Index::from(db.search(&q))
        };
        search.dirty = false;
    }

    let results: Vec<Item> = search.results.iter().cloned().collect();
    let shift = ui.window.modifiers().shift;
    let mut action = None;
    let query_empty = search.query.is_empty();

    ui.scroll_view(bounds(body).bg(colors::BG), scroll, |ui| {
        if results.is_empty() {
            ui.text(
                if query_empty {
                    "Type to search your library."
                } else {
                    "No matches."
                },
                style()
                    .fg(colors::TEXT_DIM)
                    .font_size(14)
                    .padl(40)
                    .padt(20)
                    .fill_width()
                    .align(Alignment::Left),
            );
            return;
        }

        let row = style()
            .pad(10)
            .padl(40)
            .padr(40)
            .fill_width()
            .radius(7)
            .align(Alignment::Left)
            .hover(colors::HOVER)
            .fg(colors::TEXT);

        for item in &results {
            match item {
                Item::Artist(name) => {
                    if ui.item(format!("{name}  ·  Artist"), false, row).clicked {
                        action = Some(Action::OpenArtist(name.clone()));
                    }
                }
                Item::Album((artist, album)) => {
                    if ui
                        .item(format!("{album}  ·  {artist}"), false, row)
                        .clicked
                    {
                        action = Some(Action::OpenArtist(artist.clone()));
                    }
                }
                Item::Song((artist, album, title, disc, num)) => {
                    if ui
                        .item(format!("{title}  ·  {artist} · {album}"), false, row)
                        .clicked
                    {
                        if let Some(song) = find_song(db, artists, artist, album, *disc, *num) {
                            action = Some(if shift {
                                Action::Append(song)
                            } else {
                                Action::Play(song)
                            });
                        }
                    }
                }
            }
        }
    });

    action
}

pub fn find_song(
    db: &Database,
    artists: &[String],
    artist: &str,
    album: &str,
    disc: u8,
    num: u8,
) -> Option<Song> {
    if !artists.iter().any(|a| a == artist) {
        return None;
    }
    for al in db.albums_by_artist(artist) {
        if al.title != album {
            continue;
        }
        for song in &al.songs {
            if song.disc_number == disc && song.track_number == num {
                return Some(song.clone());
            }
        }
    }
    None
}

pub fn on_backspace(search: &mut Search, window: &neoui::Window, shift: bool) {
    const INITIAL_DELAY_MS: u128 = 400;
    const REPEAT_MS: u128 = 40;

    let down = window.is_down(Key::Backspace);
    let edge = window.pressed(Key::Backspace);
    let control = window.modifiers().ctrl;

    if !down {
        search.backspace_held_since = None;
        search.backspace_last_tick = None;
        return;
    }

    let now = Instant::now();
    let should_delete = if edge {
        search.backspace_held_since = Some(now);
        search.backspace_last_tick = Some(now);
        true
    } else if let Some(held_since) = search.backspace_held_since {
        if held_since.elapsed().as_millis() < INITIAL_DELAY_MS {
            false
        } else {
            let last = search.backspace_last_tick.unwrap_or(held_since);
            if last.elapsed().as_millis() >= REPEAT_MS {
                search.backspace_last_tick = Some(now);
                true
            } else {
                false
            }
        }
    } else {
        search.backspace_held_since = Some(now);
        search.backspace_last_tick = Some(now);
        true
    };

    if !should_delete || search.query.is_empty() {
        return;
    }

    if shift && control {
        search.query.clear();
    } else if control {
        let trim = search.query.trim_end();
        if let Some(end) = trim.chars().rev().position(|c| c == ' ') {
            let keep = trim.chars().count().saturating_sub(end);
            search.query = trim.chars().take(keep).collect();
        } else {
            search.query.clear();
        }
    } else {
        search.query.pop();
    }
    search.dirty = true;
}

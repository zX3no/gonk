use crate::search;
use crate::theme::colors;
use mu_core::Song;
use mu_core::vdb::{Database, Item};
use neoui::*;
use std::time::Instant;

const PALETTE_W: i32 = 560;
const INPUT_H: i32 = 44;
const ROW_H: i32 = 36;
const MAX_VISIBLE: i32 = 10;
const MAX_RESULTS: usize = 40;
/// Above normal content / toast; below context menu.
const DEPTH: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    RescanDatabase,
}

struct CommandDef {
    id: CommandId,
    label: &'static str,
    detail: &'static str,
    /// Lowercase tokens matched against the filter after `>`.
    keywords: &'static [&'static str],
}

const COMMANDS: &[CommandDef] = &[CommandDef {
    id: CommandId::RescanDatabase,
    label: "Rescan Database",
    detail: "Refresh the music library from disk",
    keywords: &[
        "rescan", "refresh", "scan", "update", "library", "database", "reload",
    ],
}];

pub struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub dirty: bool,
    backspace_held_since: Option<Instant>,
    backspace_last_tick: Option<Instant>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
            dirty: false,
            backspace_held_since: None,
            backspace_last_tick: None,
        }
    }

    /// Open in command mode (`>` prefilled), like VS Code Ctrl+Shift+P / command palette.
    pub fn open_commands(&mut self) {
        self.open = true;
        self.query = ">".to_string();
        self.selected = 0;
        self.dirty = true;
        self.backspace_held_since = None;
        self.backspace_last_tick = None;
    }

    /// Open in song-search mode (empty query).
    pub fn open_search(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.dirty = true;
        self.backspace_held_since = None;
        self.backspace_last_tick = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.dirty = false;
        self.backspace_held_since = None;
        self.backspace_last_tick = None;
    }

    pub fn is_command_mode(&self) -> bool {
        self.query.starts_with('>')
    }
}

pub enum Action {
    RescanDatabase,
    /// Start playback and append tracks to the explicit queue.
    PlayAndQueue {
        play: Vec<Song>,
        play_index: usize,
        queue_add: Vec<Song>,
    },
    OpenArtist(String),
    Close,
}

enum Entry {
    Command(CommandId, String, String),
    Song(Item),
    Artist(String),
    Album(String, String),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    palette: &mut CommandPalette,
    db: &Database,
    artists: &[String],
    // Main content area (excludes sidebar / player bar) to center within.
    body: Rect,
    shift: bool,
) -> Option<Action> {
    if !palette.open {
        return None;
    }

    let entries = build_entries(palette, db);
    if palette.selected >= entries.len() && !entries.is_empty() {
        palette.selected = entries.len() - 1;
    }
    if entries.is_empty() {
        palette.selected = 0;
    }

    let visible = (entries.len() as i32).min(MAX_VISIBLE);
    let list_h = if entries.is_empty() {
        ROW_H
    } else {
        visible * ROW_H
    };
    let panel_h = INPUT_H + 8 + list_h + 8;
    let panel_w = PALETTE_W.min(body.width - 40).max(280);
    let panel_top = (body.height / 6).max(48);

    let (display, placeholder) = if palette.query.is_empty() {
        (
            "Search…  Enter play+queue · Shift+Enter artist · > commands".to_string(),
            true,
        )
    } else {
        (palette.query.clone(), false)
    };

    let mut action = None;
    let mut panel = Rect::default();

    // Overlay the main body so the panel is cross-aligned to that region's center.
    ui.place_down(
        flow()
            .bounds(body)
            .children_center()
            .depth(DEPTH)
            .padt(panel_top as usize),
        |ui| {
            // Chrome at full panel size (padding must not shrink the painted panel).
            let chrome = ui.rect(
                rect()
                    .width(panel_w)
                    .height(panel_h)
                    .bg(colors::PANEL_RAISED)
                    .border(colors::LINE)
                    .radius(10),
            );
            panel = chrome.bounds;

            ui.place_down(flow().bounds(chrome.bounds).pad(8), |ui| {
                ui.text(
                    display,
                    text()
                        .fillw()
                        .height(INPUT_H - 8)
                        .padlr(12)
                        .bg(colors::PANEL)
                        .border(colors::ACCENT)
                        .radius(7)
                        .fg(if placeholder {
                            colors::TEXT_DIM
                        } else {
                            colors::TEXT
                        })
                        .font_size(14)
                        .content(Alignment::Left),
                );

                ui.gap(4);

                if entries.is_empty() {
                    let hint = if palette.is_command_mode() {
                        "No matching commands"
                    } else if palette.query.is_empty() {
                        "Type to search songs, or > for commands"
                    } else {
                        "No matching songs"
                    };
                    ui.text(
                        hint,
                        text()
                            .fillw()
                            .height(ROW_H)
                            .padl(12)
                            .fg(colors::TEXT_DIM)
                            .font_size(13)
                            .content(Alignment::Left),
                    );
                    return;
                }

                let scroll_offset = palette
                    .selected
                    .saturating_sub(MAX_VISIBLE as usize - 1)
                    .min(entries.len().saturating_sub(visible as usize));

                let row = rect()
                    .fillw()
                    .height(ROW_H - 2)
                    .radius(6)
                    .hover(colors::HOVER)
                    .selected(colors::ACCENT_DIM)
                    .bg(colors::PANEL_RAISED);

                for idx in scroll_offset..scroll_offset + visible as usize {
                    let Some(entry) = entries.get(idx) else {
                        break;
                    };
                    let selected = idx == palette.selected;
                    let (title, subtitle) = entry_labels(entry);
                    let state = ui.rect(row.is_selected(selected));

                    ui.place_down(flow().bounds(state.bounds).padlr(12), |ui| {
                        let title_h = if subtitle.is_empty() {
                            state.bounds.height
                        } else {
                            state.bounds.height / 2 + 4
                        };
                        ui.text(
                            title,
                            text()
                                .fillw()
                                .height(title_h)
                                .fg(if selected {
                                    colors::ACCENT_BRIGHT
                                } else {
                                    colors::TEXT
                                })
                                .font_size(13)
                                .content(Alignment::Left),
                        );
                        if !subtitle.is_empty() {
                            ui.text(
                                subtitle,
                                text()
                                    .fillw()
                                    .height(state.bounds.height / 2)
                                    .fg(colors::TEXT_MUTED)
                                    .font_size(11)
                                    .content(Alignment::Left),
                            );
                        }
                    });

                    if state.clicked {
                        palette.selected = idx;
                        action = activate_entry(entry, db, artists, shift);
                    }
                }
            });
        },
    );

    // Click outside the panel dismisses it.
    if ui.window.mouse_released(Mouse::Left) && !ui.mouse_position().intersects(panel) {
        if action.is_none() {
            action = Some(Action::Close);
        }
    }

    action
}

pub fn try_activate(
    palette: &CommandPalette,
    db: &Database,
    artists: &[String],
    shift: bool,
) -> Option<Action> {
    let entries = build_entries(palette, db);
    let entry = entries.get(palette.selected)?;
    activate_entry(entry, db, artists, shift)
}

pub fn move_selection(palette: &mut CommandPalette, db: &Database, delta: i32) {
    let len = build_entries(palette, db).len();
    if len == 0 {
        palette.selected = 0;
        return;
    }
    let cur = palette.selected as i32;
    let next = (cur + delta).rem_euclid(len as i32) as usize;
    palette.selected = next;
}

pub fn on_text_input(palette: &mut CommandPalette, chars: &[char]) {
    for c in chars {
        if !c.is_control() {
            palette.query.push(*c);
            palette.dirty = true;
            palette.selected = 0;
        }
    }
}

pub fn on_backspace(palette: &mut CommandPalette, window: &neoui::Window, shift: bool) {
    const INITIAL_DELAY_MS: u128 = 400;
    const REPEAT_MS: u128 = 40;

    let down = window.is_down(Key::Backspace);
    let edge = window.pressed(Key::Backspace);
    let control = window.modifiers().ctrl;

    if !down {
        palette.backspace_held_since = None;
        palette.backspace_last_tick = None;
        return;
    }

    let now = Instant::now();
    let should_delete = if edge {
        palette.backspace_held_since = Some(now);
        palette.backspace_last_tick = Some(now);
        true
    } else if let Some(held_since) = palette.backspace_held_since {
        if held_since.elapsed().as_millis() < INITIAL_DELAY_MS {
            false
        } else {
            let last = palette.backspace_last_tick.unwrap_or(held_since);
            if last.elapsed().as_millis() >= REPEAT_MS {
                palette.backspace_last_tick = Some(now);
                true
            } else {
                false
            }
        }
    } else {
        palette.backspace_held_since = Some(now);
        palette.backspace_last_tick = Some(now);
        true
    };

    if !should_delete || palette.query.is_empty() {
        return;
    }

    if shift && control {
        palette.query.clear();
    } else if control {
        let trim = palette.query.trim_end();
        if let Some(end) = trim.chars().rev().position(|c| c == ' ') {
            let keep = trim.chars().count().saturating_sub(end);
            palette.query = trim.chars().take(keep).collect();
        } else {
            palette.query.clear();
        }
    } else {
        palette.query.pop();
    }
    palette.dirty = true;
    palette.selected = 0;
}

fn build_entries(palette: &CommandPalette, db: &Database) -> Vec<Entry> {
    if palette.is_command_mode() {
        let filter = palette.query[1..].trim().to_lowercase();
        COMMANDS
            .iter()
            .filter(|cmd| {
                if filter.is_empty() {
                    return true;
                }
                let label = cmd.label.to_lowercase();
                label.contains(&filter)
                    || cmd
                        .keywords
                        .iter()
                        .any(|k| k.contains(&filter) || filter.contains(k))
            })
            .map(|cmd| Entry::Command(cmd.id, cmd.label.to_string(), cmd.detail.to_string()))
            .collect()
    } else {
        let q = palette.query.trim();
        if q.is_empty() {
            return Vec::new();
        }
        db.search(q)
            .into_iter()
            .take(MAX_RESULTS)
            .filter_map(|item| match item {
                Item::Song(_) => Some(Entry::Song(item)),
                Item::Artist(name) => Some(Entry::Artist(name)),
                Item::Album((artist, album)) => Some(Entry::Album(artist, album)),
            })
            .collect()
    }
}

fn entry_labels(entry: &Entry) -> (String, String) {
    match entry {
        Entry::Command(_, label, detail) => (label.clone(), detail.clone()),
        Entry::Song(Item::Song((artist, album, title, _, _))) => {
            (title.clone(), format!("{artist}  ·  {album}"))
        }
        Entry::Song(_) => ("Song".into(), String::new()),
        Entry::Artist(name) => (name.clone(), "Artist".into()),
        Entry::Album(artist, album) => (album.clone(), format!("{artist}  ·  Album")),
    }
}

fn activate_entry(entry: &Entry, db: &Database, artists: &[String], shift: bool) -> Option<Action> {
    match entry {
        Entry::Command(CommandId::RescanDatabase, _, _) => Some(Action::RescanDatabase),
        Entry::Song(Item::Song((artist, album, _, disc, num))) => {
            // Shift+Enter → artist page; Enter → play + add to queue.
            if shift {
                return Some(Action::OpenArtist(artist.clone()));
            }
            let song = search::find_song(db, artists, artist, album, *disc, *num)?;
            let (play, play_index) = discography_from(db, artists, artist, &song.path);
            Some(Action::PlayAndQueue {
                play,
                play_index,
                queue_add: vec![song],
            })
        }
        Entry::Song(_) => None,
        Entry::Artist(name) => {
            if shift {
                return Some(Action::OpenArtist(name.clone()));
            }
            let play = if artists.iter().any(|a| a == name) {
                db.albums_by_artist(name)
                    .into_iter()
                    .flat_map(|a| a.songs.clone())
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if play.is_empty() {
                Some(Action::OpenArtist(name.clone()))
            } else {
                Some(Action::PlayAndQueue {
                    queue_add: play.clone(),
                    play,
                    play_index: 0,
                })
            }
        }
        Entry::Album(artist, album) => {
            if shift {
                return Some(Action::OpenArtist(artist.clone()));
            }
            let album_songs = search::album_songs(db, artists, artist, album);
            if album_songs.is_empty() {
                return Some(Action::OpenArtist(artist.clone()));
            }
            let start_path = album_songs[0].path.clone();
            let (play, play_index) = discography_from(db, artists, artist, &start_path);
            Some(Action::PlayAndQueue {
                play,
                play_index,
                queue_add: album_songs,
            })
        }
    }
}

/// Full artist discography for playback, starting at `path` when possible.
fn discography_from(
    db: &Database,
    artists: &[String],
    artist: &str,
    path: &str,
) -> (Vec<Song>, usize) {
    if !artists.iter().any(|a| a == artist) {
        return (Vec::new(), 0);
    }
    let play: Vec<Song> = db
        .albums_by_artist(artist)
        .into_iter()
        .flat_map(|a| a.songs.clone())
        .collect();
    let play_index = play.iter().position(|s| s.path == path).unwrap_or(0);
    (play, play_index)
}

use crate::Mode;
use crate::context_menu::{self, ContextMenu, MenuCommand};
use crate::selection::PathSelection;
use crate::theme::colors;
use mu_core::playlist::Playlist;
use mu_core::{Index, Song};
use neoui::*;

pub enum Action {
    OpenDetail(String),
    Back,
    /// Replace playback and play from index (double-click).
    Play {
        songs: Vec<Song>,
        index: usize,
    },
}

pub fn draw_list(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    lists: &Index<Playlist>,
    menu: &mut ContextMenu,
    scroll: &mut Scroll,
) -> Option<Action> {
    let mut action = None;

    ui.scroll(bounds(rect).bg(colors::BG), scroll, |ui| {
        ui.text(
            "Playlists",
            style()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(34)
                .padb(8)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            "Saved lists · right-click to play, queue, or delete",
            style()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .padb(16)
                .fill_width()
                .align(Alignment::Left),
        );

        if lists.is_empty() {
            ui.text(
                "No playlists yet. Right-click the queue and choose Save as playlist.",
                style()
                    .fg(colors::TEXT_DIM)
                    .font_size(14)
                    .padl(40)
                    .fill_width()
                    .align(Alignment::Left),
            );
            return;
        }

        let row = style()
            .pad(12)
            .padl(40)
            .padr(40)
            .fill_width()
            .radius(7)
            .align(Alignment::Left)
            .hover(colors::HOVER)
            .fg(colors::TEXT);

        for p in lists.iter() {
            let name = p.name();
            let count = p.songs.len();
            let label = ui.fmt(format_args!("{name}  ·  {count} songs"));
            let state = ui.item(label, row);
            if state.clicked {
                action = Some(Action::OpenDetail(name.to_string()));
            }
            if let Some((mx, my)) = context_menu::right_click_at(ui, state.bounds) {
                let mut entries = Vec::new();
                if !p.songs.is_empty() {
                    entries.push((
                        "Play".into(),
                        MenuCommand::Play {
                            songs: p.songs.clone(),
                            index: 0,
                        },
                    ));
                    entries.push((
                        "Add to queue".into(),
                        MenuCommand::AddToQueue(p.songs.clone()),
                    ));
                }
                let sep = entries.len();
                entries.push((
                    "Delete playlist".into(),
                    MenuCommand::DeletePlaylist(name.to_string()),
                ));
                if sep > 0 {
                    menu.open_at_with_sep(mx, my, entries, sep);
                } else {
                    menu.open_at(mx, my, entries);
                }
            }
        }
    });
    action
}

pub fn draw_detail(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    name: &str,
    lists: &Index<Playlist>,
    playing_path: Option<&str>,
    selection: &mut PathSelection,
    menu: &mut ContextMenu,
    scroll: &mut Scroll,
) -> Option<Action> {
    let Some(playlist) = lists.iter().find(|p| p.name() == name) else {
        return Some(Action::Back);
    };
    let songs = &playlist.songs;
    let ordered: Vec<&str> = songs.iter().map(|s| s.path.as_str()).collect();
    let shift = ui.window.modifiers().shift;
    let ctrl = ui.window.modifiers().ctrl;
    let mut action = None;

    ui.scroll(bounds(rect).bg(colors::BG), scroll, |ui| {
        if ui
            .item(
                "← Back",
                style()
                    .padl(40)
                    .padt(20)
                    .fg(colors::TEXT_MUTED)
                    .hover(colors::HOVER)
                    .align(Alignment::Left),
            )
            .clicked
        {
            action = Some(Action::Back);
        }

        ui.text(
            name.to_string(),
            style()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(12)
                .padb(2)
                .fill_width()
                .align(Alignment::Left),
        );
        let txt = ui.fmt(format_args!("{} songs · right-click a track", songs.len()));
        ui.text(
            txt,
            style()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .padb(16)
                .fill_width()
                .align(Alignment::Left),
        );

        let row = style()
            .pad(8)
            .padl(40)
            .padr(40)
            .fill_width()
            .radius(6)
            .align(Alignment::Left)
            .hover(colors::HOVER)
            .fg(colors::TEXT)
            .selected(colors::ACCENT_DIM);

        for (i, song) in songs.iter().enumerate() {
            let is_playing = playing_path == Some(song.path.as_str());
            let is_selected = selection.contains(&song.path);
            // ASCII only — default UI font has no ♪ glyph (rendered as ?).
            let mark = if is_playing { "> " } else { "  " };
            let label = ui.fmt(format_args!(
                "{mark}{}  ·  {}  ·  {}",
                song.title, song.artist, song.album
            ));
            let state = ui.item(label, row.is_selected(is_selected));
            if state.double_clicked {
                selection.select_only(song.path.clone());
                action = Some(Action::Play {
                    songs: songs.iter().cloned().collect(),
                    index: i,
                });
            } else if state.clicked {
                selection.click(&ordered, song.path.clone(), shift, ctrl);
            }
            if let Some((mx, my)) = context_menu::right_click_at(ui, state.bounds) {
                if !selection.contains(&song.path) {
                    selection.select_only(song.path.clone());
                }
                let selected = selection.collect_songs(songs.as_slice());
                let n = selected.len();
                let add_label = if n <= 1 {
                    "Add to queue".to_string()
                } else {
                    format!("Add {n} to queue")
                };
                menu.open_at_with_sep(
                    mx,
                    my,
                    vec![
                        (
                            "Play".into(),
                            MenuCommand::Play {
                                songs: songs.iter().cloned().collect(),
                                index: i,
                            },
                        ),
                        (add_label, MenuCommand::AddToQueue(selected)),
                        (
                            "Add all to queue".into(),
                            MenuCommand::AddToQueue(songs.iter().cloned().collect()),
                        ),
                        (
                            "Delete playlist".into(),
                            MenuCommand::DeletePlaylist(name.to_string()),
                        ),
                    ],
                    3,
                );
            }
        }
    });

    let _ = Mode::Playlist;
    action
}

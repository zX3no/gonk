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
    scroll: &mut usize,
) -> Option<Action> {
    let names: Vec<(String, usize, Vec<Song>)> = lists
        .iter()
        .map(|p| {
            (
                p.name().to_string(),
                p.songs.len(),
                p.songs.iter().cloned().collect(),
            )
        })
        .collect();
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

        if names.is_empty() {
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

        for (name, count, songs) in &names {
            let state = ui.item(format!("{name}  ·  {count} songs"), row);
            if state.clicked {
                action = Some(Action::OpenDetail(name.clone()));
            }
            if let Some((mx, my)) = context_menu::right_click_at(ui, state.rect) {
                let mut entries = Vec::new();
                if !songs.is_empty() {
                    entries.push((
                        "Play".into(),
                        MenuCommand::Play {
                            songs: songs.clone(),
                            index: 0,
                        },
                    ));
                    entries.push((
                        "Add to queue".into(),
                        MenuCommand::AddToQueue(songs.clone()),
                    ));
                }
                let sep = entries.len();
                entries.push((
                    "Delete playlist".into(),
                    MenuCommand::DeletePlaylist(name.clone()),
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
    scroll: &mut usize,
) -> Option<Action> {
    let songs: Vec<Song> = lists
        .iter()
        .find(|p| p.name() == name)
        .map(|p| p.songs.iter().cloned().collect())
        .unwrap_or_default();
    let ordered: Vec<String> = songs.iter().map(|s| s.path.clone()).collect();
    let name_owned = name.to_string();
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
            name_owned.clone(),
            style()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(12)
                .padb(2)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            format!("{} songs · right-click a track", songs.len()),
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

        let playlist_name = name_owned.clone();

        for (i, song) in songs.iter().enumerate() {
            let is_playing = playing_path == Some(song.path.as_str());
            let is_selected = selection.contains(&song.path);
            // ASCII only — default UI font has no ♪ glyph (rendered as ?).
            let mark = if is_playing { "> " } else { "  " };
            let label = format!(
                "{mark}{}  ·  {}  ·  {}",
                song.title, song.artist, song.album
            );
            let state = ui.item(label, row.is_selected(is_selected));
            if state.double_clicked {
                selection.select_only(song.path.clone());
                action = Some(Action::Play {
                    songs: songs.clone(),
                    index: i,
                });
            } else if state.clicked {
                selection.click(&ordered, song.path.clone(), shift, ctrl);
            }
            if let Some((mx, my)) = context_menu::right_click_at(ui, state.rect) {
                if !selection.contains(&song.path) {
                    selection.select_only(song.path.clone());
                }
                let selected = selection.collect_songs(&songs);
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
                                songs: songs.clone(),
                                index: i,
                            },
                        ),
                        (add_label, MenuCommand::AddToQueue(selected)),
                        (
                            "Add all to queue".into(),
                            MenuCommand::AddToQueue(songs.clone()),
                        ),
                        (
                            "Delete playlist".into(),
                            MenuCommand::DeletePlaylist(playlist_name.clone()),
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

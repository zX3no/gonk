use crate::theme::colors;
use crate::Mode;
use mu_core::playlist::Playlist;
use mu_core::{Index, Song};
use neoui::*;

pub enum Action {
    OpenDetail(String),
    Back,
    Play { songs: Vec<Song>, index: usize },
    Append(Song),
}

pub fn draw_list(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    lists: &Index<Playlist>,
    scroll: &mut usize,
) -> Option<Action> {
    let names: Vec<(String, usize)> = lists
        .iter()
        .map(|p| (p.name().to_string(), p.songs.len()))
        .collect();
    let mut action = None;

    ui.scroll_view(bounds(rect).bg(colors::BG), scroll, |ui| {
        ui.text(
            "Playlists",
            style()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(34)
                .padb(20)
                .fill_width()
                .align(Alignment::Left),
        );

        if names.is_empty() {
            ui.text(
                "No playlists yet.",
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

        for (name, count) in &names {
            if ui
                .item(format!("{name}  ·  {count} songs"), false, row)
                .clicked
            {
                action = Some(Action::OpenDetail(name.clone()));
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
    selected_path: &mut Option<String>,
    scroll: &mut usize,
) -> Option<Action> {
    let songs: Vec<Song> = lists
        .iter()
        .find(|p| p.name() == name)
        .map(|p| p.songs.iter().cloned().collect())
        .unwrap_or_default();
    let name_owned = name.to_string();
    let shift = ui.window.modifiers().shift;
    let mut action = None;

    ui.scroll_view(bounds(rect).bg(colors::BG), scroll, |ui| {
        if ui
            .item(
                "← Back",
                false,
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
                .font_size(32)
                .padl(40)
                .padt(12)
                .padb(4)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            format!("{} songs", songs.len()),
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
            let is_selected = selected_path.as_deref() == Some(song.path.as_str());
            let mark = if is_playing { "♪ " } else { "  " };
            let label = format!(
                "{mark}{}  ·  {}  ·  {}",
                song.title, song.artist, song.album
            );
            let state = ui.item(label, is_selected, row);
            if state.clicked {
                *selected_path = Some(song.path.clone());
                if shift {
                    action = Some(Action::Append(song.clone()));
                } else if state.double_clicked {
                    action = Some(Action::Play {
                        songs: songs.clone(),
                        index: i,
                    });
                }
            }
        }
    });

    let _ = Mode::Playlist;
    action
}

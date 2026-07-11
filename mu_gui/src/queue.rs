use crate::theme::colors;
use mu_core::{Index, Song};
use neoui::*;

pub enum Action {
    PlayIndex(usize),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    songs: &Index<Song>,
    selected_path: &mut Option<String>,
    scroll: &mut usize,
) -> Option<Action> {
    let tracks: Vec<Song> = songs.iter().cloned().collect();
    let playing_path = songs.selected().map(|s| s.path.clone());
    let mut action = None;

    ui.scroll_view(bounds(rect).bg(colors::BG), scroll, |ui| {
        ui.text(
            "Queue",
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
            format!("{} tracks", tracks.len()),
            style()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .padb(16)
                .fill_width()
                .align(Alignment::Left),
        );

        if tracks.is_empty() {
            ui.text(
                "Queue is empty.",
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
            .pad(8)
            .padl(40)
            .padr(40)
            .fill_width()
            .radius(6)
            .align(Alignment::Left)
            .hover(colors::HOVER)
            .fg(colors::TEXT)
            .selected(colors::ACCENT_DIM);

        for (i, song) in tracks.iter().enumerate() {
            let is_playing = playing_path.as_deref() == Some(song.path.as_str());
            let is_selected = selected_path.as_deref() == Some(song.path.as_str());
            let mark = if is_playing { "♪ " } else { "  " };
            let label = format!("{mark}{}. {}  ·  {}", i + 1, song.title, song.artist);
            let state = ui.item(label, is_selected, row);
            if state.clicked {
                *selected_path = Some(song.path.clone());
                if state.double_clicked {
                    action = Some(Action::PlayIndex(i));
                }
            }
        }
    });
    action
}

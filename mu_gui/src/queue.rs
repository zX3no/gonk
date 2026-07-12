use crate::context_menu::{self, ContextMenu, MenuCommand};
use crate::selection::PathSelection;
use crate::theme::colors;
use mu_core::{Index, Song};
use neoui::*;

pub enum Action {
    PlayIndex(usize),
}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    queue: &Index<Song>,
    playing_path: Option<&str>,
    selection: &mut PathSelection,
    menu: &mut ContextMenu,
    scroll: &mut usize,
) -> Option<Action> {
    let tracks: Vec<Song> = queue.iter().cloned().collect();
    let ordered: Vec<String> = tracks.iter().map(|s| s.path.clone()).collect();
    let playing = playing_path.map(|s| s.to_string());
    let shift = ui.window.modifiers().shift;
    let ctrl = ui.window.modifiers().ctrl;
    let mut action = None;

    ui.scroll(bounds(rect).bg(colors::BG), scroll, |ui| {
        ui.text(
            "Queue",
            style()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(34)
                .padb(4)
                .fill_width()
                .align(Alignment::Left),
        );
        ui.text(
            format!(
                "{} track{}  ·  right-click a song for actions",
                tracks.len(),
                if tracks.len() == 1 { "" } else { "s" }
            ),
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
                    .padt(4)
                    .fill_width()
                    .align(Alignment::Left),
            );
            ui.text(
                "Right-click songs in the library and choose Add to queue.",
                style()
                    .fg(colors::TEXT_DIM)
                    .font_size(13)
                    .padl(40)
                    .padt(6)
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
            let is_playing = playing.as_deref() == Some(song.path.as_str());
            let is_selected = selection.contains(&song.path);
            // ASCII only — default UI font has no ♪ glyph (rendered as ?).
            let mark = if is_playing { "> " } else { "  " };
            let label = format!(
                "{mark}{}. {}  ·  {}  ·  {}",
                i + 1,
                song.title,
                song.artist,
                song.album
            );

            let state = ui.item(label, is_selected, row);
            if state.double_clicked {
                selection.select_only(song.path.clone());
                action = Some(Action::PlayIndex(i));
            } else if state.clicked {
                selection.click(&ordered, song.path.clone(), shift, ctrl);
            }
            if let Some((mx, my)) = context_menu::right_click_at(ui, state.rect) {
                if !selection.contains(&song.path) {
                    selection.select_only(song.path.clone());
                }
                let idxs: Vec<usize> = selection
                    .paths()
                    .iter()
                    .filter_map(|p| ordered.iter().position(|o| o == p))
                    .collect();
                let n = idxs.len().max(1);
                let remove_label = if n == 1 {
                    "Remove".to_string()
                } else {
                    format!("Remove {n} tracks")
                };

                let mut entries = vec![
                    (
                        "Play".into(),
                        MenuCommand::Play {
                            songs: tracks.clone(),
                            index: i,
                        },
                    ),
                    (
                        remove_label,
                        MenuCommand::RemoveFromQueue(if idxs.is_empty() { vec![i] } else { idxs }),
                    ),
                ];
                if i > 0 {
                    entries.push(("Move up".into(), MenuCommand::MoveUp(i)));
                }
                if i + 1 < tracks.len() {
                    entries.push(("Move down".into(), MenuCommand::MoveDown(i)));
                }
                let sep_at = entries.len();
                entries.push(("Clear queue".into(), MenuCommand::ClearQueue));
                entries.push((
                    "Clear except playing".into(),
                    MenuCommand::ClearExceptPlaying,
                ));
                entries.push(("Save as playlist".into(), MenuCommand::SaveQueueAsPlaylist));
                menu.open_at_with_sep(mx, my, entries, sep_at);
            }
        }

        ui.gap(32);
    });
    action
}

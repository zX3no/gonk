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
    scroll: &mut Scroll,
) -> Option<Action> {
    let queue_len = queue.len();
    let ordered: Vec<&str> = queue.iter().map(|s| s.path.as_str()).collect();
    let shift = ui.window.modifiers().shift;
    let ctrl = ui.window.modifiers().ctrl;
    let mut action = None;

    ui.scroll(flow().bounds(rect).bg(colors::BG), scroll, |ui| {
        ui.text(
            "Queue",
            text()
                .fg(colors::TEXT)
                .font_size(28)
                .padl(40)
                .padt(34)
                .padb(4)
                .fillw()
                .content(Alignment::Left),
        );
        let txt = ui.fmt(format_args!(
            "{} track{}  ·  right-click a song for actions",
            queue_len,
            if queue_len == 1 { "" } else { "s" }
        ));
        ui.text(
            txt,
            text()
                .fg(colors::TEXT_MUTED)
                .font_size(13)
                .padl(40)
                .padb(16)
                .fillw()
                .content(Alignment::Left),
        );

        if queue.is_empty() {
            ui.text(
                "Queue is empty.",
                text()
                    .fg(colors::TEXT_DIM)
                    .font_size(14)
                    .padl(40)
                    .padt(4)
                    .fillw()
                    .content(Alignment::Left),
            );
            ui.text(
                "Right-click songs in the library and choose Add to queue.",
                text()
                    .fg(colors::TEXT_DIM)
                    .font_size(13)
                    .padl(40)
                    .padt(6)
                    .fillw()
                    .content(Alignment::Left),
            );
            return;
        }

        let row = text()
            .pad(8)
            .padlr(40)
            .fillw()
            .radius(6)
            .content(Alignment::Left)
            .hover(colors::HOVER)
            .fg(colors::TEXT)
            .selected(colors::ACCENT_DIM);

        for (i, song) in queue.iter().enumerate() {
            let is_playing = playing_path == Some(song.path.as_str());
            let is_selected = selection.contains(&song.path);
            // ASCII only — default UI font has no ♪ glyph (rendered as ?).
            let mark = if is_playing { "> " } else { "  " };
            let label = ui.fmt(format_args!(
                "{mark}{}. {}  ·  {}  ·  {}",
                i + 1,
                song.title,
                song.artist,
                song.album
            ));

            let state = ui.text(label, row.is_selected(is_selected));
            if state.double_clicked {
                selection.select_only(song.path.clone());
                action = Some(Action::PlayIndex(i));
            } else if state.clicked {
                selection.click(&ordered, song.path.clone(), shift, ctrl);
            }
            if let Some((mx, my)) = context_menu::right_click_at(ui, state.bounds) {
                if !selection.contains(&song.path) {
                    selection.select_only(song.path.clone());
                }
                let idxs: Vec<usize> = selection
                    .paths()
                    .iter()
                    .filter_map(|p| ordered.iter().position(|o| *o == p))
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
                            songs: queue.iter().cloned().collect(),
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
                if i + 1 < queue_len {
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

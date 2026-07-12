//! Floating right-click context menu.

use crate::theme::colors;
use mu_core::Song;
use neoui::*;

const MENU_W: i32 = 220;
const ROW_H: i32 = 32;
const PAD: i32 = 4;

#[derive(Clone)]
pub enum MenuCommand {
    /// Play a temporary session (does not touch the queue).
    Play { songs: Vec<Song>, index: usize },
    /// Append to the explicit queue.
    AddToQueue(Vec<Song>),
    /// Queue: remove indices.
    RemoveFromQueue(Vec<usize>),
    /// Queue: reorder.
    MoveUp(usize),
    MoveDown(usize),
    ClearQueue,
    ClearExceptPlaying,
    SaveQueueAsPlaylist,
}

struct Entry {
    label: String,
    command: MenuCommand,
    /// Visual separator before this row.
    separator_before: bool,
}

pub struct ContextMenu {
    open: bool,
    x: i32,
    y: i32,
    entries: Vec<Entry>,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self {
            open: false,
            x: 0,
            y: 0,
            entries: Vec::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
        self.entries.clear();
    }

    pub fn open_at(&mut self, x: i32, y: i32, entries: Vec<(String, MenuCommand)>) {
        if entries.is_empty() {
            return;
        }
        self.open = true;
        self.x = x;
        self.y = y;
        self.entries = entries
            .into_iter()
            .map(|(label, command)| Entry {
                label,
                command,
                separator_before: false,
            })
            .collect();
    }

    /// Like `open_at`, but marks the entry at `sep_before_index` with a separator line above it.
    pub fn open_at_with_sep(
        &mut self,
        x: i32,
        y: i32,
        entries: Vec<(String, MenuCommand)>,
        sep_before_index: usize,
    ) {
        self.open_at(x, y, entries);
        if let Some(e) = self.entries.get_mut(sep_before_index) {
            e.separator_before = true;
        }
    }
}

/// Draw the menu on top of the UI. Returns a chosen command (and closes the menu).
pub fn draw(ui: &mut FrameContext<'_, '_>, menu: &mut ContextMenu) -> Option<MenuCommand> {
    if !menu.open {
        return None;
    }

    let (win_w, win_h) = ui.window.content_size();
    let win_w = win_w as i32;
    let win_h = win_h as i32;

    let mut content_h = PAD * 2;
    for (i, e) in menu.entries.iter().enumerate() {
        if e.separator_before && i > 0 {
            content_h += 9;
        }
        content_h += ROW_H;
    }

    let x = menu.x.clamp(8, (win_w - MENU_W - 8).max(8));
    let y = menu.y.clamp(8, (win_h - content_h - 8).max(8));
    let panel = Rect::new(x, y, MENU_W, content_h);
    let depth = 4;

    ui.paint_rect(
        panel,
        style()
            .bg(colors::PANEL_RAISED)
            .border(colors::LINE)
            .radius(8)
            .depth(depth),
    );

    let mut cy = y + PAD;
    let mut chosen = None;

    for (i, entry) in menu.entries.iter().enumerate() {
        if entry.separator_before && i > 0 {
            ui.paint_rect(
                Rect::new(x + 10, cy + 3, MENU_W - 20, 1),
                style().bg(colors::LINE).depth(depth),
            );
            cy += 9;
        }

        let row = Rect::new(x + PAD, cy, MENU_W - PAD * 2, ROW_H - 2);
        let hovered = ui.hovered(row);
        if hovered {
            ui.paint_rect(
                row,
                style().bg(colors::HOVER).radius(5).depth(depth),
            );
        }

        ui.paint_text(
            entry.label.clone(),
            row.x + 12,
            row.y,
            row.width - 24,
            row.height,
            colors::TEXT,
            0,
            13,
            Alignment::Left,
            Padding::default(),
            depth,
        );

        if ui.clicked(row) {
            chosen = Some(entry.command.clone());
        }
        cy += ROW_H;
    }

    // Click outside dismisses (don't steal the click that opened us on the same frame —
    // open happens on right-click; left-click outside closes).
    if ui.window.mouse_clicked(Mouse::Left, Rect::new(0, 0, win_w, win_h))
        && !ui.mouse_position().intersects(panel)
        && chosen.is_none()
    {
        menu.close();
        return None;
    }
    if ui.window.mouse_clicked(Mouse::Right, Rect::new(0, 0, win_w, win_h))
        && !ui.mouse_position().intersects(panel)
    {
        // Another right-click outside: close; the view may open a new menu same frame.
        menu.close();
    }

    if let Some(cmd) = chosen {
        menu.close();
        return Some(cmd);
    }
    None
}

/// Screen position for a right-click that hit `rect`, if any.
pub fn right_click_at(ui: &FrameContext<'_, '_>, rect: Rect) -> Option<(i32, i32)> {
    if ui.window.mouse_clicked(Mouse::Right, rect) {
        let p = ui.mouse_position();
        Some((p.x, p.y))
    } else {
        None
    }
}

use crate::theme::colors;
use neoui::*;
use std::time::{Duration, Instant};

const TOAST_W: i32 = 320;
const TOAST_H: i32 = 72;
const MARGIN: i32 = 16;
const DEFAULT_TTL: Duration = Duration::from_secs(4);

pub struct Toast {
    pub message: String,
    pub detail: String,
    shown_at: Instant,
    ttl: Duration,
}

impl Toast {
    pub fn new(message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            detail: detail.into(),
            shown_at: Instant::now(),
            ttl: DEFAULT_TTL,
        }
    }

    pub fn expired(&self) -> bool {
        self.shown_at.elapsed() >= self.ttl
    }
}

/// Draw a bottom-right toast above the player bar. Returns true if the toast was dismissed.
pub fn draw(ui: &mut FrameContext<'_, '_>, toast: &Toast, above_y: i32, window_w: i32) -> bool {
    let x = window_w - TOAST_W - MARGIN;
    let y = above_y - TOAST_H - MARGIN;
    if x < 0 || y < 0 {
        return false;
    }

    let rect = Rect::new(x, y, TOAST_W, TOAST_H);
    let depth = 2;

    ui.paint_rect(
        rect,
        style()
            .bg(colors::PANEL_RAISED)
            .border(colors::LINE)
            .radius(10)
            .depth(depth),
    );

    // Accent strip on the left.
    ui.paint_rect(
        Rect::new(rect.x, rect.y, 4, rect.height),
        style().bg(colors::ACCENT).radius(10).depth(depth),
    );

    ui.paint_text(
        toast.message.clone(),
        rect.x + 16,
        rect.y + 14,
        rect.width - 28,
        20,
        colors::TEXT,
        0,
        14,
        Alignment::Left,
        Padding::default(),
        depth,
    );

    if !toast.detail.is_empty() {
        ui.paint_text(
            toast.detail.clone(),
            rect.x + 16,
            rect.y + 38,
            rect.width - 28,
            18,
            colors::TEXT_MUTED,
            0,
            12,
            Alignment::Left,
            Padding::default(),
            depth,
        );
    }

    // Click to dismiss.
    ui.clicked(rect)
}

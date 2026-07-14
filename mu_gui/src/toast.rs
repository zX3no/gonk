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
pub fn draw(ui: &mut FrameContext<'_, '_>, toast: &'_ Toast, above_y: i32, window_w: i32) -> bool {
    let x = window_w - TOAST_W - MARGIN;
    let y = above_y - TOAST_H - MARGIN;
    if x < 0 || y < 0 {
        return false;
    }

    let rect = Rect::new(x, y, TOAST_W, TOAST_H);
    let message = toast.message.clone();
    let detail = toast.detail.clone();

    let (bar, body) = ui.split_rect_h(rect, 4);

    // Accent strip is outside the place frame, so depth must be set explicitly.
    ui.paint_rect(bar, bg(colors::ACCENT).depth(2));

    ui.place_right(
        bounds(body)
            .bg(colors::PANEL_RAISED)
            .border(colors::LINE)
            .cross_align(CrossAlign::Center)
            .depth(2),
        |ui| {
            let content_h = if detail.is_empty() { 20 } else { 44 };
            let content = ui.rect(style().fill_width().height(content_h)).rect;

            ui.place_down(bounds(content), |ui| {
                ui.text(
                    message,
                    style()
                        .padl(12)
                        .fg(colors::TEXT)
                        .font_size(14)
                        .fill_width()
                        .height(20)
                        .align(Alignment::Left),
                );

                if !detail.is_empty() {
                    ui.gap(4);
                    ui.text(
                        detail,
                        style()
                            .padl(12)
                            .fg(colors::TEXT_MUTED)
                            .font_size(12)
                            .fill_width()
                            .height(18)
                            .align(Alignment::Left),
                    );
                }
            });
        },
    );

    ui.clicked(rect)
}

use crate::theme::colors;
use neoui::*;

pub enum Action {}

pub fn draw(
    ui: &mut FrameContext<'_, '_>,
    rect: Rect,
    _artists: &[String],
    _queue_title: Option<&str>,
    _scroll: &mut usize,
) -> Option<Action> {
    ui.paint_rect(rect, style().bg(colors::BG));
    None
}

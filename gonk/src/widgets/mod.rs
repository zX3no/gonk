#![allow(unused)]

mod guage;
mod list;
mod table;

pub use guage::*;
pub use list::*;
pub use table::*;

use tui::layout::{Margin, Rect};

pub fn centered_rect(width: u16, height: u16, area: Rect) -> Option<Rect> {
    let w = area.width / 2;
    let h = area.height / 2;

    let mut rect = area.inner(&Margin {
        vertical: h.saturating_sub(height / 2),
        horizontal: w.saturating_sub(width / 2),
    });

    rect.width = width;
    rect.height = height;

    if area.height < rect.height || area.width < rect.width {
        None
    } else {
        Some(rect)
    }
}

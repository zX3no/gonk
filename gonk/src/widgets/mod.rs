#![allow(unused)]
#![allow(clippy::pedantic)]

mod guage;
mod list;
mod table;

pub use guage::*;
pub use list::*;
pub use table::*;

use tui::layout::{Margin, Rect};

pub fn centered_rect(width: u16, height: u16, area: Rect) -> Option<Rect> {
    let v = area.height / 2;
    let h = area.width / 2;

    let mut rect = area.inner(&Margin {
        vertical: v.saturating_sub(height / 2),
        horizontal: h.saturating_sub(width / 2),
    });

    rect.width = width;
    rect.height = height;

    if area.height < rect.height || area.width < rect.width {
        None
    } else {
        Some(rect)
    }
}

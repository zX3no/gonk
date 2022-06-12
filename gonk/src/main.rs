use app::App;
use gonk_core::sqlite;
use std::io::{Result, Stdout};
use tui::{
    backend::CrosstermBackend,
    layout::{Margin, Rect},
};

mod app;
mod widgets;

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

pub type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    unsafe {
        //Initialize database.
        sqlite::CONN = sqlite::open_database();
    }

    match App::new() {
        Ok(mut app) => app.run(),
        Err(err) => {
            return if err.is_empty() {
                Ok(())
            } else {
                Ok(println!("{}", err))
            }
        }
    }
}

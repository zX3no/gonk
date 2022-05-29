use app::App;
use gonk_core::sqlite;
use std::io::{Result, Stdout};
use tui::backend::CrosstermBackend;

mod app;
mod widgets;

pub type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    //Initialize database.
    unsafe {
        sqlite::CONN = sqlite::open_database();
    }

    if let Some(mut app) = App::new() {
        app.run()?;
    }

    Ok(())
}

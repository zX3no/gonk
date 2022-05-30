use app::App;
use gonk_core::sqlite;
use std::io::Result;

mod app;
mod widgets;

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

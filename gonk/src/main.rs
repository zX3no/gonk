use app::App;
use gonk_core::sqlite;
use std::io::Result;

mod app;
mod widgets;

fn main() -> Result<()> {
    unsafe {
        //Initialize database.
        sqlite::CONN = sqlite::open_database();
    }

    match App::new() {
        Ok(mut app) => app.run(),
        Err(err) => return Ok(println!("{}", err)),
    }
}

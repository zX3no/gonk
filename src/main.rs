mod app;
mod database;
mod list;
mod musiclibrary;
mod player;

use app::App;

fn main() {
    let mut app = App::new();
    app.run().unwrap();
}

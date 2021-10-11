mod app;
mod database;
mod index;
mod list;
mod musiclibrary;
mod player;

use app::App;

fn main() {
    let mut app = App::new();
    app.run().unwrap();
}

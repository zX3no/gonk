mod app;
mod database;
mod index;
mod list;
mod musiclibrary;
mod player;

use app::App;
use index::get_artists;
use player::Player;

fn main() {
    let mut app = App::new();
    app.run().unwrap();
    let artists = get_artists();

    // let path = &artists["Badbadnotgood"]
    //     .album("Talk Memory")
    //     .unwrap()
    //     .track(7)
    //     .unwrap()
    //     .path;

    // Player::play(path);
}

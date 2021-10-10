mod database;
mod index;
mod player;

use std::path::PathBuf;

use index::get_artists;
use player::Player;

fn main() {
    let artists = get_artists();

    let path = &artists["Badbadnotgood"]
        .album("Talk Memory")
        .unwrap()
        .at(2)
        .unwrap()
        .path;

    Player::play(path);
}

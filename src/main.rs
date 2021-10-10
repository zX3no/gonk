mod database;
mod index;
mod player;

use index::get_artists;
use player::Player;

fn main() {
    let artists = get_artists();

    let path = &artists["Badbadnotgood"]
        .album("Talk Memory")
        .unwrap()
        .track(7)
        .unwrap()
        .path;

    Player::play(path);
}

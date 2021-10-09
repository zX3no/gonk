mod database;
mod index;
use index::get_artists;

fn main() {
    let artists = get_artists();
}

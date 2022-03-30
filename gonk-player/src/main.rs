use std::{path::PathBuf, thread};

use gonk_player::Player;
use gonk_types::Song;

fn main() {
    let mut player = Player::new(10);
    let song = Song::from(&PathBuf::from(
        r"D:\Music\Floating Points, Pharoah Sanders & The London Symphony Orchestra\Promises\03. Floating Points - Movement 3.flac",
    ));
    player.add_songs(&[song]);
    player.play();
    thread::park();
}

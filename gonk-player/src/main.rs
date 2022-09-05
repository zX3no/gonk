use gonk_player::*;
use std::{thread, time::Duration};

fn main() {
    let player = Player::new();
    // player.play_song(
    //     r"D:\OneDrive\Music\Duster\Contemporary Movement\03. Diamond.flac".to_string(),
    //     State::Playing,
    // );
    player.play_song(
        r"D:\OneDrive\Music\Nirvana\Nirvana (Album)\01 You Know You're Right.flac".to_string(),
        State::Playing,
    );

    thread::park();
}

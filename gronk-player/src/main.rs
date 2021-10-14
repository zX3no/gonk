#![allow(dead_code)]
use player::Player;
use std::thread;

mod player;
fn main() {
    //todo talk to the player using channels
    thread::spawn(|| {
        let player = Player::new();
        player.run()
    });
}

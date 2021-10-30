#![allow(dead_code)]
use player::Player;
use std::thread;
use std::time::Duration;
use std::{panic, process};

mod event_handler;
mod player;
mod queue;

fn main() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    let player = Player::new();

    player.next();
    // player.next();
    // dbg!(player.get_seeker());

    thread::park();
}

#![allow(dead_code)]
use player::Player;
use std::thread;
use std::{panic, process};

mod event_handler;
mod player;

fn thread_panic() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));
}
fn main() {
    thread_panic();

    let player = Player::new();

    player.next();
    player.next();

    thread::park();
}

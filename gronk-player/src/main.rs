#![allow(dead_code)]
use event_handler::{Event, EventHandler};
use player::Player;
use std::sync::mpsc::channel;
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

    thread::park();
}

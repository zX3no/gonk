#![allow(dead_code)]
use player::Player;
use std::{
    borrow::BorrowMut,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use std::{panic, process};
mod player;

fn multi_thread_panic() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));
}
fn main() {
    multi_thread_panic();

    let player = Arc::new(RwLock::new(Player::new()));
    let mut p = player.clone();

    thread::spawn(move || {
        p.borrow_mut().write().unwrap().run();
    });

    let p = player.clone();

    thread::park();
}

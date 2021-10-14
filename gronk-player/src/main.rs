#![allow(dead_code)]
use player::Player;
use std::{
    borrow::BorrowMut,
    sync::{Arc, Mutex, RwLock},
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
    let p = player.clone();

    thread::spawn(move || {
        //this is does not work
        p.write().unwrap().run();
    });

    thread::park();
}

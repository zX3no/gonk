#![allow(dead_code)]
use player::{Event, Player};
use std::sync::mpsc::channel;
use std::thread;
use std::{panic, process};

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

    let (tx, rx) = channel();

    tx.send(Event::Next).unwrap();

    thread::spawn(move || {
        let mut player = Player::new();
        loop {
            player.update();
            match rx.recv().unwrap() {
                Event::Next => player.next(),
                _ => (),
            }
        }
    });

    thread::park();
}

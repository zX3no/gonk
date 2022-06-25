#![allow(unused)]
use gonk_database::{query::*, *};

fn add_path() {
    let mut db = Database::default();
    db.add_path("D:\\OneDrive\\Music");

    loop {
        if db.state() == State::NeedsUpdate {
            break;
        }
    }
}

fn main() {
    init();

    playlist::add("test", &[1, 2]);
    let a = playlist::get("test");
    println!("{:?}", a);
}

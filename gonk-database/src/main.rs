#![allow(unused)]
use gonk_database::*;

fn main() {
    unsafe {
        init();
        dbg!(&SETTINGS);
    }
}

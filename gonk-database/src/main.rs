#![allow(unused)]
use gonk_database::*;

fn main() {
    settings::init();
    settings::update_volume(45);
    settings::update_queue(Vec::new(), 3232323.0);

    unsafe {
        dbg!(&settings::SETTINGS);
    }
}

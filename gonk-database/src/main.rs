#![allow(unused)]
use gonk_database::*;

fn main() {
    init();

    unsafe {
        update_volume(15);
        update_queue(&Vec::new(), 15, 1.323259);
        update_output_device("OUT 1-4");
        dbg!(&SETTINGS);
    }
}

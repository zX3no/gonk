#![allow(unused)]
use gonk_database::*;

fn main() {
    init();

    bench_slow(|| {
        let _items = albums();
    })
}

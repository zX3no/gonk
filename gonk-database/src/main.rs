use gonk_database::*;

fn main() {
    init();

    dbg!(albums());

    // bench_slow(|| {
    //     par_albums();
    // });
}

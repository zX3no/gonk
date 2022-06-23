use gonk_database::query::*;
use gonk_database::*;

fn main() {
    reset();
    init().unwrap();
    add_folder("D:\\OneDrive\\Music");

    dbg!(total_songs());
}

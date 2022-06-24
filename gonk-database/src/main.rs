use gonk_database::{query::*, *};

fn main() {
    init().unwrap();
    let mut db = Database::default();
    db.add_path("D:\\OneDrive\\Music");

    loop {
        if db.state() == State::NeedsUpdate {
            break;
        }
    }

    let a = songs_from_ids(&[200, 323, 1000, 2, 1021]);
    println!("{:?}", a);
}

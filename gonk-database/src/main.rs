use gonk_database::*;
fn main() {
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("{panic_info}");
        std::process::exit(1);
    }));

    // let result = create_database_single("D:\\OneDrive\\Music");
    let db = read_database().unwrap();
    let artists = db.artists();
    dbg!(artists);
}

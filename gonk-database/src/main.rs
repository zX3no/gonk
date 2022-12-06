use gonk_database::{vdb::Database, *};

static mut DB: Lazy<Database> = Lazy::new(|| vdb::create().unwrap());

fn main() {
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("{panic_info}");
        std::process::exit(1);
    }));

    unsafe {
        dbg!(vdb::artist(&DB, "Iglooghost"));
    }
}

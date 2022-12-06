use gonk_database::*;

static mut DB: Lazy<Database> = Lazy::new(|| read_database().unwrap());

fn main() {
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("{panic_info}");
        std::process::exit(1);
    }));

    unsafe {
        dbg!(artist(&DB, "Iglooghost"));
    }
}

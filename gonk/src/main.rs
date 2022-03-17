use app::App;
use gonk_database::{Database, Toml, CONFIG_DIR, TOML_DIR};
use std::io::Result;

mod app;
mod index;

fn main() -> Result<()> {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let mut toml = Toml::new()?;

    //Handle arguments
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(first) = args.first() {
        match first as &str {
            "add" => {
                if let Some(dir) = args.get(1..) {
                    let dir = dir.join(" ");
                    toml.add_path(dir);
                }
            }
            "config" => {
                println!("Gonk directory:  {}", CONFIG_DIR.to_string_lossy());
                println!("Config file:     {}", TOML_DIR.to_string_lossy());
                return Ok(());
            }
            "reset" | "rm" => {
                Database::delete();
                println!("Database reset!");
                return Ok(());
            }
            "help" => {
                println!("Usage");
                println!("    gonk [<options> <args>]\n");
                println!("Options");
                println!("    add       Add music to the library");
                println!("    reset     Reset the database");
                println!();
                return Ok(());
            }
            _ => {
                println!("Invalid command.");
                return Ok(());
            }
        }
    }

    App::new().run()?;

    Ok(())
}

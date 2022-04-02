use app::App;
use gonk_database::{Database, Toml, CONFIG_DIR};
use std::io::Result;
mod app;

#[macro_use]
extern crate log;

fn main() -> Result<()> {
    env_logger::init();

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(first) = args.first() {
        match first as &str {
            "add" => {
                if let Some(dir) = args.get(1..) {
                    let dir = dir.join(" ");
                    Toml::new()?.add_path(dir);
                }
            }
            "config" => {
                println!("Gonk directory:  {}", CONFIG_DIR.to_string_lossy());
                return Ok(());
            }
            "reset" | "rm" => {
                Database::delete();
                println!("Database reset!");
                return Ok(());
            }
            "help" => {
                println!("Usage");
                println!("   gonk [<command> <args>]");
                println!();
                println!("Options");
                println!("   add <path>  Add music to the library");
                println!("   config      Locates the config directory");
                println!("   reset       Reset the database");
                println!();
                return Ok(());
            }
            _ => {
                println!("Invalid command.");
                return Ok(());
            }
        }
    }

    info!("app running");
    App::new().run()?;

    Ok(())
}

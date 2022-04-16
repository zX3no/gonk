use app::App;
use gonk_database::{Database, GONK_DIR};
use std::io::Result;
mod app;
mod widget;

fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    optick::start_capture();
    optick::event!("main");

    let args: Vec<_> = std::env::args().skip(1).collect();
    let db = Database::new().unwrap();

    if let Some(first) = args.first() {
        match first as &str {
            "add" => {
                if let Some(dir) = args.get(1..) {
                    let dir = dir.join(" ");
                    db.add_dirs(&[dir]);
                }
            }
            "config" => {
                println!("Gonk directory:  {}", GONK_DIR.to_string_lossy());
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

    App::new().run()?;

    // #[cfg(debug_assertions)]
    // optick::stop_capture("gonk");

    Ok(())
}

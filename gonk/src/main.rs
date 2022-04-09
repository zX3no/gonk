use app::App;
use gonk_database::{Database, Toml, GONK_DIR};
use std::io::Result;
mod app;
mod widget;

//TODO: there are so many instances of Toml::new()
//really need to clean them up.

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(first) = args.first() {
        match first as &str {
            "add" => {
                if let Some(dir) = args.get(1..) {
                    let dir = dir.join(" ");
                    Toml::new().add_path(dir);
                }
            }
            "config" => {
                println!("Gonk directory:  {}", GONK_DIR.to_string_lossy());
                return Ok(());
            }
            "reset" | "rm" => {
                Toml::new().reset();
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

    Ok(())
}

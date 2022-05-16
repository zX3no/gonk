use app::App;
use gonk_core::{sqlite, Toml};
use std::io::Result;
mod app;
mod widgets;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut toml = Toml::new();
    if let Some(first) = args.first() {
        match first as &str {
            "add" => {
                if let Some(dir) = args.get(1..) {
                    let dir = dir.join(" ");
                    toml.add_path(dir);
                }
            }
            "reset" => {
                sqlite::reset();
                toml.reset();
                println!("Reset database!");
                return Ok(());
            }
            "help" | "--help" => {
                println!("Usage");
                println!("   gonk [<command> <args>]");
                println!();
                println!("Options");
                println!("   add   <path>  Add music to the library");
                println!("   reset         Reset the database");
                println!();
                return Ok(());
            }
            _ => {
                println!("Invalid command.");
                return Ok(());
            }
        }
    }

    //Initialize database.
    unsafe {
        sqlite::CONN = sqlite::open_database();
    }

    App::new(&mut toml).run(toml)?;

    Ok(())
}

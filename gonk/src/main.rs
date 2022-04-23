use app::App;
use gonk_server::Client;
use std::io::Result;
mod app;
mod widget;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut client = Client::new().sync();

    if let Some(first) = args.first() {
        match first as &str {
            "add" => {
                if let Some(path) = args.get(1..) {
                    let path = path.join(" ");
                    client.add_path(path);
                }
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

    App::new(client).run()?;

    Ok(())
}

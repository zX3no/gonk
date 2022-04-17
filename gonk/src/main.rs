use app::App;
use gonk_core::GONK_DIR;
use gonk_server::Client;
use std::io::Result;
mod app;
mod widget;

fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    optick::start_capture();
    optick::event!("main");

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
            "config" => {
                println!("Gonk directory:  {}", GONK_DIR.to_string_lossy());
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

    App::new(client).run()?;

    // #[cfg(debug_assertions)]
    // optick::stop_capture("gonk");

    Ok(())
}

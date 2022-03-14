use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gonk_database::{Database, Toml, CONFIG_DIR, TOML_DIR};
use std::io::{stdout, Result};
use tui::{backend::CrosstermBackend, Terminal};
mod app;
mod index;

#[cfg(windows)]
#[derive(Debug, Clone)]
enum HotkeyEvent {
    PlayPause,
    Next,
    Prev,
    VolUp,
    VolDown,
}

fn main() -> Result<()> {
    //TODO: make sure toml file isn't empty and that there are no songs left over in database
    let db = Database::new().unwrap();
    let mut toml = Toml::new().unwrap();

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
                drop(db);
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

    //Make sure the database and toml file share the same directories
    db.add_music(&toml.paths());

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    //Get ready for rendering and input
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let mut app = App::new();

    app.run(&mut terminal)?;

    //Cleanup terminal for exit
    disable_raw_mode()?;
    terminal.show_cursor()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}

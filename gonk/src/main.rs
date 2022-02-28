use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use gonk_database::{Database, Toml, CONFIG_DIR, TOML_DIR};
use std::{
    io::{stdout, Result},
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
#[cfg(windows)]
use {
    std::sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    win_hotkey::*,
};

mod app;
mod index;
mod ui;

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
    terminal.backend_mut().enable_raw_mode()?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let app = App::new(&db);

    run_app(&mut terminal, app)?;

    //Cleanup terminal for exit
    terminal.backend_mut().disable_raw_mode()?;
    terminal.show_cursor()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16);

    #[cfg(windows)]
    let tx = register_hotkeys();

    loop {
        #[cfg(windows)]
        if let Ok(recv) = tx.try_recv() {
            match recv {
                HotkeyEvent::VolUp => app.queue.volume_up(),
                HotkeyEvent::VolDown => app.queue.volume_down(),
                HotkeyEvent::PlayPause => app.queue.play_pause(),
                HotkeyEvent::Prev => app.queue.prev(),
                HotkeyEvent::Next => app.queue.next(),
            }
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    //I don't like bool returns like this but ...?
                    if app.input(key.code, key.modifiers) {
                        return Ok(());
                    };
                }
                Event::Mouse(mouse) => app.mouse(mouse),
                _ => (),
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

#[cfg(windows)]
fn register_hotkeys() -> Receiver<HotkeyEvent> {
    let (rx, tx) = mpsc::sync_channel(1);
    let rx = Arc::new(rx);
    std::thread::spawn(move || {
        let mut hk = Listener::<HotkeyEvent>::new();
        let ghk = Toml::new().unwrap().global_hotkey;

        hk.register_hotkey(
            ghk.volume_up.modifiers(),
            ghk.volume_up.key(),
            HotkeyEvent::VolUp,
        );
        hk.register_hotkey(
            ghk.volume_down.modifiers(),
            ghk.volume_down.key(),
            HotkeyEvent::VolDown,
        );
        hk.register_hotkey(
            ghk.previous.modifiers(),
            ghk.previous.key(),
            HotkeyEvent::Prev,
        );
        hk.register_hotkey(ghk.next.modifiers(), ghk.next.key(), HotkeyEvent::Next);
        hk.register_hotkey(modifiers::SHIFT, keys::ESCAPE, HotkeyEvent::PlayPause);
        loop {
            if let Some(event) = hk.listen() {
                rx.send(event.clone()).unwrap();
            }
        }
    });
    tx
}

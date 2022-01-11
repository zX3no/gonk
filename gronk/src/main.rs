use app::App;
use crossterm::{
    event::{self, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gronk_database::Database;
use std::{
    io::{stdout, Result},
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

mod app;
mod index;
mod ui;

fn main() -> Result<()> {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        execute!(stdout(), LeaveAlternateScreen).unwrap();
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let db = Database::new().unwrap();

    //check if user wants to add new database
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(first) = args.first() {
        if first == "add" {
            if let Some(dir) = args.get(1..) {
                db.add_dir(&dir.join(" "));
            }
        }
    }

    let app = App::new(&db);

    run_app(&mut terminal, app)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if let KeyCode::Char('c') = key.code {
                        if key.modifiers == KeyModifiers::CONTROL {
                            return Ok(());
                        } else {
                            app.handle_char('c', key.modifiers);
                        }
                    } else {
                        app.input(key.code, key.modifiers);
                    }
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

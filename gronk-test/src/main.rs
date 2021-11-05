use app::App;
use std::{
    io::stdout,
    panic, process,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, Event as CTEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod music;
mod ui;

enum Event {
    Input(KeyEvent),
    Tick,
}

fn main() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    let backend = CrosstermBackend::new(stdout());

    let mut terminal = Terminal::new(backend).unwrap();

    execute!(stdout(), EnterAlternateScreen).unwrap();
    enable_raw_mode().unwrap();

    // Setup input handling
    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(1000);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).unwrap() {
                if let CTEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    let mut app = App::new();

    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        match rx.recv().unwrap() {
            Event::Input(event) => match event.code {
                KeyCode::Char('c') => {
                    if event.modifiers == KeyModifiers::CONTROL {
                        disable_raw_mode().unwrap();
                        execute!(
                            terminal.backend_mut(),
                            LeaveAlternateScreen,
                            DisableMouseCapture
                        )
                        .unwrap();
                        terminal.show_cursor().unwrap();
                        break;
                    }
                }
                KeyCode::Char('j') => app.browser_down(),
                KeyCode::Char('k') => app.browser_up(),
                KeyCode::Char('h') => app.browser_prev(),
                KeyCode::Char('l') => app.browser_next(),
                KeyCode::Tab => app.ui_toggle(),
                _ => (),
            },
            Event::Tick => {
                // app.on_tick();
            }
        }
    }
}

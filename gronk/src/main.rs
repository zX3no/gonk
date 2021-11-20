use app::App;
use std::{
    io::stdout,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event as CTEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod modes;
mod ui;

enum Event {
    Input(KeyEvent),
    Tick,
}

fn main() {
    execute!(stdout(), EnterAlternateScreen).unwrap();
    enable_raw_mode().unwrap();
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();

    let mut app = App::new();

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(50);

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

    loop {
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        match rx.recv().unwrap() {
            Event::Input(event) => {
                if let KeyCode::Char('c') = event.code {
                    if event.modifiers == KeyModifiers::CONTROL {
                        disable_raw_mode().unwrap();
                        execute!(terminal.backend_mut(), LeaveAlternateScreen,).unwrap();
                        terminal.show_cursor().unwrap();
                        break;
                    } else {
                        app.handle_input('c', event.modifiers);
                    }
                } else {
                    match event.code {
                        KeyCode::Char('u') => {
                            app.update();
                        }
                        KeyCode::Char(c) => app.handle_input(c, event.modifiers),
                        KeyCode::Down => app.down(),
                        KeyCode::Up => app.up(),
                        KeyCode::Left => app.browser_prev(),
                        KeyCode::Right => app.browser_next(),
                        KeyCode::Enter => app.on_enter(),
                        KeyCode::Tab => app.on_tab(),
                        KeyCode::Backspace => app.on_backspace(event.modifiers),
                        _ => (),
                    }
                }
            }
            Event::Tick => {
                app.on_tick();
            }
        }
    }
}

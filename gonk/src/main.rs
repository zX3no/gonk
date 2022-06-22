use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use queue::Queue;
use search::Mode as SearchMode;
use search::Search;
use sqlite::Database;
use sqlite::State;
use static_init::dynamic;
use status_bar::StatusBar;
use std::time::Instant;
use std::{
    io::{stdout, Stdout},
    path::PathBuf,
    time::Duration,
};
use tui::layout::*;
use tui::{backend::CrosstermBackend, style::Color, *};

mod browser;
mod queue;
mod search;
mod sqlite;
mod status_bar;
mod widgets;

type Frame<'a> = tui::Frame<'a, CrosstermBackend<Stdout>>;

pub struct Colors {
    pub number: Color,
    pub name: Color,
    pub album: Color,
    pub artist: Color,
    pub seeker: Color,
}

impl Colors {
    const fn new() -> Self {
        Self {
            number: Color::Green,
            name: Color::Cyan,
            album: Color::Magenta,
            artist: Color::Blue,
            seeker: Color::White,
        }
    }
}

const COLORS: Colors = Colors::new();

#[dynamic]
static GONK_DIR: PathBuf = {
    let gonk = if cfg!(windows) {
        PathBuf::from(&std::env::var("APPDATA").unwrap())
    } else {
        PathBuf::from(&std::env::var("HOME").unwrap()).join(".config")
    }
    .join("gonk");

    if !gonk.exists() {
        std::fs::create_dir_all(&gonk).unwrap();
    }
    gonk
};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Browser,
    Queue,
    Search,
}

pub trait Input {
    fn up(&mut self);
    fn down(&mut self);
    fn left(&mut self);
    fn right(&mut self);
}

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        disable_raw_mode().unwrap();
        execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture,
    )
    .unwrap();
    enable_raw_mode().unwrap();
    terminal.clear().unwrap();

    sqlite::initialize_database();

    let mut browser = Browser::new();
    let mut queue = Queue::new(15);
    let mut search = Search::new();
    let mut status_bar = StatusBar::new();
    let mut db = Database::default();
    let mut mode = Mode::Browser;
    let mut busy = false;
    let mut last_tick = Instant::now();

    loop {
        //Update
        {
            if last_tick.elapsed() >= Duration::from_millis(200) {
                //Update the status_bar at a constant rate.
                status_bar::update(&mut status_bar, busy, &queue);
                last_tick = Instant::now();
            }

            queue.player.update();

            match db.state() {
                State::Busy => busy = true,
                State::Idle => busy = false,
                State::NeedsUpdate => {
                    browser::refresh(&mut browser);
                    search::refresh(&mut search);
                }
            }
        }

        //Draw
        terminal
            .draw(|f| {
                let area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(2), Constraint::Length(3)])
                    .split(f.size());

                let (top, bottom) = if status_bar.hidden {
                    (f.size(), area[1])
                } else {
                    (area[0], area[1])
                };

                match mode {
                    Mode::Browser => browser::draw(&mut browser, top, f),
                    Mode::Queue => queue::draw(&mut queue, f, None),
                    Mode::Search => search::draw(&mut search, top, f),
                };

                if mode != Mode::Queue {
                    status_bar::draw(&mut status_bar, bottom, f, busy, &queue);
                }
            })
            .unwrap();

        let get_input = search.mode == SearchMode::Search && mode == Mode::Search;

        let input = match mode {
            Mode::Browser => &mut browser as &mut dyn Input,
            Mode::Queue => &mut queue as &mut dyn Input,
            Mode::Search => &mut search as &mut dyn Input,
        };

        if event::poll(Duration::default()).unwrap() {
            match event::read().unwrap() {
                Event::Key(event) => {
                    let shift = event.modifiers == KeyModifiers::SHIFT;
                    let control = event.modifiers == KeyModifiers::CONTROL;

                    match event.code {
                        KeyCode::Char('c') if control => break,
                        KeyCode::Char(c) if get_input => {
                            //Handle ^W as control backspace.
                            if control && c == 'w' {
                                search::on_backspace(&mut search, true)
                            } else {
                                search.query_changed = true;
                                search.query.push(c);
                            }
                        }
                        // KeyCode::Char(c)
                        //     if playlist.input_mode() && mode == Mode::Playlist =>
                        // {
                        //     playlist.on_key(c)
                        // }
                        KeyCode::Char(' ') => queue.player.toggle_playback(),
                        KeyCode::Char('c') if shift => {
                            queue.player.clear_except_playing();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('c') => {
                            queue.player.clear();
                            queue.ui.select(Some(0));
                        }
                        KeyCode::Char('x') => queue::delete(&mut queue),
                        KeyCode::Char('u') if mode == Mode::Browser => {
                            db.add_paths(&[String::from("D:/OneDrive/Music")])
                        }
                        KeyCode::Char('q') => queue.player.seek_by(-10.0),
                        KeyCode::Char('e') => queue.player.seek_by(10.0),
                        KeyCode::Char('a') => queue.player.prev_song(),
                        KeyCode::Char('d') => queue.player.next_song(),
                        KeyCode::Char('w') => queue.player.volume_up(),
                        KeyCode::Char('s') => queue.player.volume_down(),
                        KeyCode::Char('r') => queue.player.randomize(),
                        //TODO: Rework mode changing buttons
                        KeyCode::Char('`') => {
                            status_bar.hidden = !status_bar.hidden;
                        }
                        // KeyCode::Char(',') => mode = Mode::Playlist,
                        // KeyCode::Char('.') => mode = Mode::Options,
                        KeyCode::Char('/') => mode = Mode::Search,
                        KeyCode::Tab => {
                            mode = match mode {
                                // Mode::Browser | Mode::Options => Mode::Queue,
                                Mode::Browser => Mode::Queue,
                                Mode::Queue => Mode::Browser,
                                Mode::Search => Mode::Queue,
                                // Mode::Playlist => Mode::Browser,
                            };
                        }
                        KeyCode::Esc => match mode {
                            Mode::Search => search::on_escape(&mut search, &mut mode),
                            // Mode::Options => mode = Mode::Queue,
                            // Mode::Playlist => playlist.on_escape(&mut mode),
                            _ => (),
                        },
                        // KeyCode::Enter if shift => match mode {
                        //     Mode::Browser => {
                        //         let songs = browser.on_enter();
                        //         playlist.add_to_playlist(&songs);
                        //         mode = Mode::Playlist;
                        //     }
                        //     Mode::Queue => {
                        //         if let Some(song) = queue.selected() {
                        //             playlist.add_to_playlist(&[song.clone()]);
                        //             mode = Mode::Playlist;
                        //         }
                        //     }
                        //     _ => (),
                        // },
                        KeyCode::Enter => match mode {
                            Mode::Browser => {
                                let songs = browser::on_enter(&browser);
                                queue.player.add_songs(&songs);
                            }
                            Mode::Queue => {
                                if let Some(i) = queue.ui.index() {
                                    queue.player.play_song(i);
                                }
                            }
                            Mode::Search => search::on_enter(&mut search, &mut queue.player),
                            // Mode::Options => options.on_enter(&mut queue.player, &mut toml),
                            // Mode::Playlist => playlist.on_enter(&mut queue.player),
                        },
                        KeyCode::Backspace => {
                            match mode {
                                Mode::Search => search::on_backspace(&mut search, control),
                                // Mode::Playlist => self.playlist.on_backspace(event.modifiers),
                                _ => (),
                            }
                        }
                        KeyCode::Up => input.up(),
                        KeyCode::Down => input.down(),
                        KeyCode::Left => input.left(),
                        KeyCode::Right => input.right(),
                        KeyCode::Char('1' | '!') => {
                            queue::constraint(&mut queue, 0, shift);
                        }
                        KeyCode::Char('2' | '@') => {
                            queue::constraint(&mut queue, 1, shift);
                        }
                        KeyCode::Char('3' | '#') => {
                            queue::constraint(&mut queue, 2, shift);
                        }
                        KeyCode::Char(c) => match c {
                            'h' => input.left(),
                            'j' => input.down(),
                            'k' => input.up(),
                            'l' => input.right(),
                            _ => (),
                        },
                        _ => (),
                    }
                }
                Event::Mouse(event) => match event.kind {
                    MouseEventKind::ScrollUp => input.up(),
                    MouseEventKind::ScrollDown => input.down(),
                    MouseEventKind::Down(_) => {
                        if let Mode::Queue = mode {
                            terminal
                                .draw(|f| queue::draw(&mut queue, f, Some(event)))
                                .unwrap();
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
}

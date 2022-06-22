use browser::Browser;
use crossterm::{event::*, terminal::*, *};
use queue::Queue;
use search::Mode as SearchMode;
use search::Search;
use sqlite::Database;
use sqlite::State;
use static_init::dynamic;
use std::time::Instant;
use std::{
    io::{stdout, Stdout},
    path::PathBuf,
    time::Duration,
};
use tui::{backend::CrosstermBackend, style::Color, *};

mod browser;
mod queue;
mod search;
mod sqlite;
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

const POLL_RATE: Duration = Duration::from_millis(4);
const TICK_RATE: Duration = Duration::from_millis(200);
const SEEK_TIME: f64 = 10.0;

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
    let mut db = Database::default();
    let mut mode = Mode::Browser;
    let mut busy = false;
    let mut last_tick = Instant::now();

    loop {
        //Update
        {
            if last_tick.elapsed() >= TICK_RATE {
                //Update the status_bar at a constant rate.
                // status_bar.update(busy, queue);
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
                match mode {
                    Mode::Browser => browser::draw(&mut browser, f.size(), f),
                    Mode::Queue => queue::draw(&mut queue, f, None),
                    Mode::Search => search::draw(&mut search, f.size(), f),
                };
            })
            .unwrap();

        let get_input = search.mode == SearchMode::Search && mode == Mode::Search;

        let input = match mode {
            Mode::Browser => &mut browser as &mut dyn Input,
            Mode::Queue => &mut queue as &mut dyn Input,
            Mode::Search => &mut search as &mut dyn Input,
        };

        if crossterm::event::poll(POLL_RATE).unwrap() {
            match event::read().unwrap() {
                Event::Key(event) => {
                    let shift = event.modifiers == KeyModifiers::SHIFT;

                    match event.code {
                        KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => break,
                        KeyCode::Char(c) if get_input => {
                            //Handle ^W as control backspace.
                            if event.modifiers == KeyModifiers::CONTROL && c == 'w' {
                                search::on_backspace(&mut search, true)
                            } else {
                                search.query_changed = true;
                                search.query.push(c);
                            }
                        }
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
                        KeyCode::Char('u') if mode == Mode::Browser => (),
                        KeyCode::Char('q') => queue.player.seek_by(-SEEK_TIME),
                        KeyCode::Char('e') => queue.player.seek_by(SEEK_TIME),
                        KeyCode::Char('a') => queue.player.prev_song(),
                        KeyCode::Char('d') => queue.player.next_song(),
                        KeyCode::Char('w') => queue.player.volume_up(),
                        KeyCode::Char('s') => queue.player.volume_down(),
                        KeyCode::Char('r') => queue.player.randomize(),
                        KeyCode::Char('/') => mode = Mode::Search,
                        KeyCode::Tab => match mode {
                            Mode::Browser => mode = Mode::Queue,
                            Mode::Queue => mode = Mode::Browser,
                            Mode::Search => mode = Mode::Queue,
                        },
                        KeyCode::Esc => match mode {
                            Mode::Search => search::on_escape(&mut search, &mut mode),
                            // Mode::Options => mode = Mode::Queue,
                            // Mode::Playlist => playlist.on_escape(&mut mode),
                            _ => (),
                        },
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
                                Mode::Search => search::on_backspace(&mut search, shift),
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
                Event::Mouse(_) => {}
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

//     match event.code {
//         KeyCode::Char(c) if self.search.input_mode() && self.mode == Mode::Search => {
//             self.search.on_key(c)
//         }
//         KeyCode::Char(c) if self.playlist.input_mode() && self.mode == Mode::Playlist => {
//             self.playlist.on_key(c)
//         }
//         KeyCode::Up => self.up(),
//         KeyCode::Down => self.down(),
//         KeyCode::Left => self.left(),
//         KeyCode::Right => self.right(),
//         KeyCode::Tab => {
//             self.mode = match self.mode {
//                 Mode::Browser | Mode::Options => Mode::Queue,
//                 Mode::Queue => Mode::Browser,
//                 Mode::Search => Mode::Queue,
//                 Mode::Playlist => Mode::Browser,
//             };
//         }
//         KeyCode::Backspace => match self.mode {
//             Mode::Search => self.search.on_backspace(event.modifiers),
//             Mode::Playlist => self.playlist.on_backspace(event.modifiers),
//             _ => (),
//         },
//         KeyCode::Enter if shift => match self.mode {
//             Mode::Browser => {
//                 let songs = self.browser.on_enter();
//                 self.playlist.add_to_playlist(&songs);
//                 self.mode = Mode::Playlist;
//             }
//             Mode::Queue => {
//                 if let Some(song) = self.queue.selected() {
//                     self.playlist.add_to_playlist(&[song.clone()]);
//                     self.mode = Mode::Playlist;
//                 }
//             }
//             _ => (),
//         },
//         KeyCode::Enter => match self.mode {
//             Mode::Browser => {
//                 let songs = self.browser.on_enter();
//                 self.queue.player.add_songs(&songs);
//             }
//             Mode::Queue => {
//                 if let Some(i) = self.queue.ui.index() {
//                     self.queue.player.play_song(i);
//                 }
//             }
//             Mode::Search => self.search.on_enter(&mut self.queue.player),
//             Mode::Options => self
//                 .options
//                 .on_enter(&mut self.queue.player, &mut self.toml),
//             Mode::Playlist => self.playlist.on_enter(&mut self.queue.player),
//         },
//         KeyCode::Esc => match self.mode {
//             Mode::Search => self.search.on_escape(&mut self.mode),
//             Mode::Options => self.mode = Mode::Queue,
//             Mode::Playlist => self.playlist.on_escape(&mut self.mode),
//             _ => (),
//         },
//         //TODO: Rework mode changing buttons
//         KeyCode::Char('`') => {
//             self.status_bar.toggle_hidden();
//         }
//         KeyCode::Char(',') => self.mode = Mode::Playlist,
//         KeyCode::Char('.') => self.mode = Mode::Options,
//         KeyCode::Char('/') => self.mode = Mode::Search,
//         KeyCode::Char('1' | '!') => {
//             queue::constraint(0, event.modifiers);
//         }
//         KeyCode::Char('2' | '@') => {
//             queue::constraint(1, event.modifiers);
//         }
//         KeyCode::Char('3' | '#') => {
//             queue::constraint(2, event.modifiers);
//         }
//         _ if hotkey.up == bind => self.up(),
//         _ if hotkey.down == bind => self.down(),
//         _ if hotkey.left == bind => self.left(),
//         _ if hotkey.right == bind => self.right(),
//         _ if hotkey.play_pause == bind => self.queue.player.toggle_playback(),
//         _ if hotkey.clear == bind => self.queue.clear(),
//         _ if hotkey.clear_except_playing == bind => {
//             self.queue.clear_except_playing();
//         }
//         _ if hotkey.refresh_database == bind && self.mode == Mode::Browser => {
//             self.db.add_paths(&self.toml.config.paths);
//         }
//         _ if hotkey.seek_backward == bind && self.mode != Mode::Search => {
//             self.queue.player.seek_by(-SEEK_TIME)
//         }
//         _ if hotkey.seek_forward == bind && self.mode != Mode::Search => {
//             self.queue.player.seek_by(SEEK_TIME)
//         }
//         _ if hotkey.previous == bind && self.mode != Mode::Search => self.queue.player.prev_song(),
//         _ if hotkey.next == bind && self.mode != Mode::Search => self.queue.player.next_song(),
//         _ if hotkey.volume_up == bind => {
//             self.queue.player.volume_up();
//             self.toml.set_volume(self.queue.player.volume);
//         }
//         _ if hotkey.volume_down == bind => {
//             self.queue.player.volume_down();
//             self.toml.set_volume(self.queue.player.volume);
//         }
//         _ if hotkey.delete == bind => match self.mode {
//             Mode::Queue => self.queue.delete(),
//             Mode::Playlist => self.playlist.delete(),
//             _ => (),
//         },
//         _ if hotkey.random == bind => self.queue.player.randomize(),
//         _ => (),
//     }

use crate::sqlite::{Database, State};
use crate::{sqlite, toml::*};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::time::Duration;
use std::time::Instant;
use std::{
    io::{stdout, Stdout},
    path::Path,
};
use tui::Terminal;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};

use self::status_bar::StatusBar;
use {browser::Browser, options::Options, playlist::Playlist, queue::Queue, search::Search};

mod browser;
mod options;
mod playlist;
mod queue;
mod search;
mod status_bar;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Mode {
    Browser,
    Queue,
    Search,
    Options,
    Playlist,
}

const TICK_RATE: Duration = Duration::from_millis(200);
const POLL_RATE: Duration = Duration::from_millis(4);
const SEEK_TIME: f32 = 10.0;

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub mode: Mode,
    queue: Queue,
    browser: Browser,
    options: Options,
    search: Search,
    playlist: Playlist,
    status_bar: StatusBar,
    toml: Toml,
    db: Database,
    busy: bool,
}

impl App {
    pub fn new() -> Result<Self, String> {
        match Toml::new().check_paths() {
            Ok(mut toml) => {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let mut db = Database::default();

                if let Some(first) = args.first() {
                    match first as &str {
                        "add" => {
                            if let Some(dir) = args.get(1..) {
                                let dir = dir.join(" ");
                                let path = Path::new(&dir);
                                if path.exists() {
                                    toml.add_path(dir.clone());
                                    db.add_paths(&[dir]);
                                } else {
                                    return Err(format!("{} is not a valid path.", dir));
                                }
                            }
                        }
                        "reset" => {
                            sqlite::reset();
                            toml.reset();
                            return Err(String::from("Files reset!"));
                        }
                        "help" | "--help" => {
                            println!("Usage");
                            println!("   gonk [<command> <args>]");
                            println!();
                            println!("Options");
                            println!("   add   <path>  Add music to the library");
                            println!("   reset         Reset the database");
                            return Err(String::new());
                        }
                        _ => return Err(String::from("Invalid command.")),
                    }
                }

                //make sure the terminal recovers after a panic
                let orig_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |panic_info| {
                    disable_raw_mode().unwrap();
                    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
                    orig_hook(panic_info);
                    std::process::exit(1);
                }));

                //Initialize the terminal and clear the screen
                let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture,
                )
                .unwrap();
                enable_raw_mode().unwrap();
                terminal.clear().unwrap();

                Ok(Self {
                    terminal,
                    mode: Mode::Browser,
                    queue: Queue::new(
                        toml.config.volume,
                        toml.colors,
                        toml.config.output_device.clone(),
                    ),
                    browser: Browser::new(),
                    options: Options::new(&mut toml),
                    search: Search::new(toml.colors).init(),
                    playlist: Playlist::new(),
                    status_bar: StatusBar::new(toml.colors),
                    busy: false,
                    db,
                    toml,
                })
            }
            Err(err) => Err(err),
        }
    }
    pub fn run(&mut self) -> std::io::Result<()> {
        let mut last_tick = Instant::now();

        loop {
            if last_tick.elapsed() >= TICK_RATE {
                //Update the status_bar at a constant rate.
                self.status_bar.update(self.busy, &self.queue);
                last_tick = Instant::now();
            }

            match self.db.state() {
                State::Busy => self.busy = true,
                State::Idle => self.busy = false,
                State::NeedsUpdate => {
                    self.browser.refresh();
                    self.search.update();
                }
            }

            self.queue.update();

            self.terminal.draw(|f| {
                let area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(2), Constraint::Length(3)])
                    .split(f.size());

                let top = if self.status_bar.is_hidden() {
                    f.size()
                } else {
                    area[0]
                };

                match self.mode {
                    Mode::Browser => self.browser.draw(top, f),
                    Mode::Queue => self.queue.draw(f),
                    Mode::Options => self.options.draw(top, f),
                    Mode::Search => self.search.draw(top, f),
                    Mode::Playlist => self.playlist.draw(top, f),
                };

                if self.mode != Mode::Queue {
                    self.status_bar.draw(area[1], f, self.busy, &self.queue);
                }
            })?;

            if crossterm::event::poll(POLL_RATE)? {
                match event::read()? {
                    Event::Key(event) => {
                        let hotkey = &self.toml.hotkey;
                        let shift = event.modifiers == KeyModifiers::SHIFT;
                        let bind = Bind {
                            key: Key::from(event.code),
                            modifiers: Modifier::from_bitflags(event.modifiers),
                        };

                        //Check if the user wants to exit.
                        if event.code == KeyCode::Char('C') && shift {
                            break;
                        } else if hotkey.quit == bind {
                            break;
                        };

                        match event.code {
                            KeyCode::Char(c)
                                if self.search.input_mode() && self.mode == Mode::Search =>
                            {
                                self.search.on_key(c)
                            }
                            KeyCode::Char(c)
                                if self.playlist.input_mode() && self.mode == Mode::Playlist =>
                            {
                                self.playlist.on_key(c)
                            }
                            KeyCode::Up => self.up(),
                            KeyCode::Down => self.down(),
                            KeyCode::Left => self.left(),
                            KeyCode::Right => self.right(),
                            KeyCode::Tab => {
                                self.mode = match self.mode {
                                    Mode::Browser | Mode::Options => Mode::Queue,
                                    Mode::Queue => Mode::Browser,
                                    Mode::Search => Mode::Queue,
                                    Mode::Playlist => Mode::Browser,
                                };
                            }
                            KeyCode::Backspace => match self.mode {
                                Mode::Search => self.search.on_backspace(event.modifiers),
                                Mode::Playlist => self.playlist.on_backspace(event.modifiers),
                                _ => (),
                            },
                            KeyCode::Enter if shift => match self.mode {
                                Mode::Browser => {
                                    let songs = self.browser.on_enter();
                                    self.playlist.add_to_playlist(&songs);
                                    self.mode = Mode::Playlist;
                                }
                                Mode::Queue => {
                                    if let Some(song) = self.queue.player.selected_song() {
                                        self.playlist.add_to_playlist(&[song.clone()]);
                                        self.mode = Mode::Playlist;
                                    }
                                }
                                _ => (),
                            },
                            KeyCode::Enter => match self.mode {
                                Mode::Browser => {
                                    let songs = self.browser.on_enter();
                                    self.queue.player.add_songs(&songs);
                                }
                                Mode::Queue => {
                                    if let Some(i) = self.queue.ui.index() {
                                        self.queue.player.play_index(i);
                                    }
                                }
                                Mode::Search => self.search.on_enter(&mut self.queue.player),
                                Mode::Options => self
                                    .options
                                    .on_enter(&mut self.queue.player, &mut self.toml),
                                Mode::Playlist => self.playlist.on_enter(&mut self.queue.player),
                            },
                            KeyCode::Esc => match self.mode {
                                Mode::Search => self.search.on_escape(&mut self.mode),
                                Mode::Options => self.mode = Mode::Queue,
                                Mode::Playlist => self.playlist.on_escape(&mut self.mode),
                                _ => (),
                            },
                            //TODO: Rework mode changing buttons
                            KeyCode::Char('`') => {
                                self.status_bar.toggle_hidden();
                            }
                            KeyCode::Char(',') => self.mode = Mode::Playlist,
                            KeyCode::Char('.') => self.mode = Mode::Options,
                            KeyCode::Char('/') => self.mode = Mode::Search,
                            KeyCode::Char('1' | '!') => {
                                self.queue.move_constraint(0, event.modifiers);
                            }
                            KeyCode::Char('2' | '@') => {
                                self.queue.move_constraint(1, event.modifiers);
                            }
                            KeyCode::Char('3' | '#') => {
                                self.queue.move_constraint(2, event.modifiers);
                            }
                            _ if hotkey.up == bind => self.up(),
                            _ if hotkey.down == bind => self.down(),
                            _ if hotkey.left == bind => self.left(),
                            _ if hotkey.right == bind => self.right(),
                            _ if hotkey.play_pause == bind => self.queue.player.toggle_playback(),
                            _ if hotkey.clear == bind => self.queue.clear(),
                            _ if hotkey.clear_except_playing == bind => {
                                self.queue.clear_except_playing();
                            }
                            _ if hotkey.refresh_database == bind && self.mode == Mode::Browser => {
                                self.db.add_paths(&self.toml.config.paths);
                            }
                            _ if hotkey.seek_backward == bind && self.mode != Mode::Search => {
                                self.queue.player.seek_by(-SEEK_TIME)
                            }
                            _ if hotkey.seek_forward == bind && self.mode != Mode::Search => {
                                self.queue.player.seek_by(SEEK_TIME)
                            }
                            _ if hotkey.previous == bind && self.mode != Mode::Search => {
                                self.queue.player.previous()
                            }
                            _ if hotkey.next == bind && self.mode != Mode::Search => {
                                self.queue.player.next()
                            }
                            _ if hotkey.volume_up == bind => {
                                self.queue.player.volume_up();
                                self.toml.set_volume(self.queue.player.get_volume());
                            }
                            _ if hotkey.volume_down == bind => {
                                self.queue.player.volume_down();
                                self.toml.set_volume(self.queue.player.get_volume());
                            }
                            _ if hotkey.delete == bind => match self.mode {
                                Mode::Queue => self.queue.delete(),
                                Mode::Playlist => self.playlist.delete(),
                                _ => (),
                            },
                            _ if hotkey.random == bind => self.queue.player.randomize(),
                            _ => (),
                        }
                    }
                    Event::Mouse(event) => match event.kind {
                        MouseEventKind::ScrollUp => self.up(),
                        MouseEventKind::ScrollDown => self.down(),
                        MouseEventKind::Down(_) => {
                            self.queue.clicked_pos = Some((event.column, event.row));
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn left(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.left(),
            Mode::Playlist => self.playlist.left(),
            _ => (),
        }
    }

    fn right(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.right(),
            Mode::Playlist => self.playlist.right(),
            _ => (),
        }
    }

    fn up(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.up(),
            Mode::Queue => self.queue.up(),
            Mode::Search => self.search.up(),
            Mode::Options => self.options.up(),
            Mode::Playlist => self.playlist.up(),
        }
    }

    fn down(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.down(),
            Mode::Queue => self.queue.down(),
            Mode::Search => self.search.down(),
            Mode::Options => self.options.down(),
            Mode::Playlist => self.playlist.down(),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
    }
}

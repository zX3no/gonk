use crate::sqlite::{Database, State};
use crate::{config::*, sqlite};
use crossbeam_channel::{unbounded, Receiver};
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

#[derive(Debug, Clone)]
enum HotkeyEvent {
    PlayPause,
    Next,
    Prev,
    VolUp,
    VolDown,
}

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
const SEEK_TIME: f64 = 10.0;

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
                            println!("Database reset!");
                            return Err(String::new());
                        }
                        "help" | "--help" => {
                            println!("Usage");
                            println!("   gonk [<command> <args>]");
                            println!();
                            println!("Options");
                            println!("   add   <path>  Add music to the library");
                            println!("   reset         Reset the database");
                            println!();
                            return Err(String::new());
                        }
                        _ => {
                            println!("Invalid command.");
                            return Err(String::new());
                        }
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
                    queue: Queue::new(toml.volume(), toml.colors),
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

        #[cfg(windows)]
        let tx = App::register_hotkeys(self.toml.clone());

        loop {
            //Update every 200ms.
            if last_tick.elapsed() >= TICK_RATE {
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
                    Mode::Options => self.options.draw(top, f, &self.toml),
                    Mode::Search => self.search.draw(top, f),
                    Mode::Playlist => self.playlist.draw(top, f),
                };

                if self.mode != Mode::Queue {
                    self.status_bar.draw(area[1], f, self.busy, &self.queue);
                }
            })?;

            #[cfg(windows)]
            if let Ok(recv) = tx.try_recv() {
                match recv {
                    HotkeyEvent::VolUp => {
                        self.queue.player.volume_up();
                        self.toml.set_volume(self.queue.player.volume);
                    }
                    HotkeyEvent::VolDown => {
                        self.queue.player.volume_down();
                        self.toml.set_volume(self.queue.player.volume);
                    }
                    HotkeyEvent::PlayPause => self.queue.player.toggle_playback(),
                    HotkeyEvent::Prev => self.queue.player.prev_song(),
                    HotkeyEvent::Next => self.queue.player.next_song(),
                }
            }

            if crossterm::event::poll(POLL_RATE)? {
                match event::read()? {
                    Event::Key(event) => {
                        let bind = Bind {
                            key: Key::from(event.code),
                            modifiers: Modifier::from_bitflags(event.modifiers),
                        };
                        let hotkey = &self.toml.hotkey;

                        if hotkey.quit.contains(&bind) {
                            break;
                        };

                        let shift = event.modifiers == KeyModifiers::SHIFT;

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
                                    if let Some(song) = self.queue.selected() {
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
                                        self.queue.player.play_song(i);
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
                            KeyCode::Char('1' | '!') => {
                                self.queue.move_constraint(0, event.modifiers);
                            }
                            KeyCode::Char('2' | '@') => {
                                self.queue.move_constraint(1, event.modifiers);
                            }
                            KeyCode::Char('3' | '#') => {
                                self.queue.move_constraint(2, event.modifiers);
                            }
                            //TODO: Add playlist mode to config file.
                            KeyCode::Char(',') => {
                                self.mode = Mode::Playlist;
                            }
                            //TODO: Add to config file.
                            KeyCode::Char('`') => {
                                self.status_bar.toggle_hidden();
                            }
                            _ if hotkey.up.contains(&bind) => self.up(),
                            _ if hotkey.down.contains(&bind) => self.down(),
                            _ if hotkey.left.contains(&bind) => match self.mode {
                                Mode::Browser => self.browser.left(),
                                Mode::Playlist => self.playlist.left(),
                                _ => (),
                            },
                            _ if hotkey.right.contains(&bind) => match self.mode {
                                Mode::Browser => self.browser.right(),
                                Mode::Playlist => self.playlist.right(),
                                _ => (),
                            },
                            _ if hotkey.play_pause.contains(&bind) => {
                                self.queue.player.toggle_playback()
                            }
                            _ if hotkey.clear.contains(&bind) => self.queue.clear(),
                            _ if hotkey.clear_except_playing.contains(&bind) => {
                                self.queue.clear_except_playing();
                            }
                            _ if hotkey.refresh_database.contains(&bind)
                                && self.mode == Mode::Browser =>
                            {
                                let paths = self.toml.paths();
                                self.db.add_paths(paths);
                            }
                            _ if hotkey.seek_backward.contains(&bind)
                                && self.mode != Mode::Search =>
                            {
                                self.queue.player.seek_by(-SEEK_TIME)
                            }
                            _ if hotkey.seek_forward.contains(&bind)
                                && self.mode != Mode::Search =>
                            {
                                self.queue.player.seek_by(SEEK_TIME)
                            }
                            _ if hotkey.previous.contains(&bind) && self.mode != Mode::Search => {
                                self.queue.player.prev_song()
                            }
                            _ if hotkey.next.contains(&bind) && self.mode != Mode::Search => {
                                self.queue.player.next_song()
                            }
                            _ if hotkey.volume_up.contains(&bind) => {
                                self.queue.player.volume_up();
                                self.toml.set_volume(self.queue.player.volume);
                            }
                            _ if hotkey.volume_down.contains(&bind) => {
                                self.queue.player.volume_down();
                                self.toml.set_volume(self.queue.player.volume);
                            }
                            _ if hotkey.search.contains(&bind) => self.mode = Mode::Search,
                            _ if hotkey.options.contains(&bind) => self.mode = Mode::Options,
                            _ if hotkey.delete.contains(&bind) => match self.mode {
                                Mode::Queue => self.queue.delete(),
                                Mode::Playlist => self.playlist.delete(),
                                _ => (),
                            },
                            _ if hotkey.random.contains(&bind) => self.queue.player.randomize(),
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

    #[cfg(windows)]
    fn register_hotkeys(toml: Toml) -> Receiver<HotkeyEvent> {
        use global_hotkeys::{keys, modifiers, Listener};
        let (rx, tx) = unbounded();
        std::thread::spawn(move || {
            let mut hk = Listener::<HotkeyEvent>::new();
            hk.register_hotkey(
                toml.global_hotkey.volume_up.modifiers(),
                toml.global_hotkey.volume_up.key(),
                HotkeyEvent::VolUp,
            );
            hk.register_hotkey(
                toml.global_hotkey.volume_down.modifiers(),
                toml.global_hotkey.volume_down.key(),
                HotkeyEvent::VolDown,
            );
            hk.register_hotkey(
                toml.global_hotkey.previous.modifiers(),
                toml.global_hotkey.previous.key(),
                HotkeyEvent::Prev,
            );
            hk.register_hotkey(
                toml.global_hotkey.next.modifiers(),
                toml.global_hotkey.next.key(),
                HotkeyEvent::Next,
            );
            hk.register_hotkey(modifiers::SHIFT, keys::ESCAPE, HotkeyEvent::PlayPause);
            drop(toml);
            loop {
                if let Some(event) = hk.listen() {
                    rx.send(event.clone()).unwrap();
                }
            }
        });
        tx
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

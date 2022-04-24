use crossbeam_channel::{unbounded, Receiver};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gonk_core::{Bind, Colors, Database, Key, Modifier, Toml};
use static_init::dynamic;
use std::io::{stdout, Stdout};
use std::time::Duration;
use std::time::Instant;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use {browser::Browser, options::Options, queue::Queue, search::Search};

mod browser;
mod options;
mod queue;
mod search;

#[derive(Debug, Clone)]
enum HotkeyEvent {
    PlayPause,
    Next,
    Prev,
    VolUp,
    VolDown,
}

#[derive(PartialEq, Debug)]
pub enum Mode {
    Browser,
    Queue,
    Search,
    Options,
}

#[dynamic]
static DB: Database = Database::default();

#[dynamic]
static COLORS: Colors = Toml::new().colors;

const TICK_RATE: Duration = Duration::from_millis(100);
const POLL_RATE: Duration = Duration::from_millis(4);
const SEEK_TIME: f64 = 10.0;

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub mode: Mode,
    queue: Queue,
    browser: Browser,
    options: Options,
    search: Search,
    db: Database,
}

impl App {
    pub fn new(toml: &mut Toml) -> Self {
        //make sure the terminal recovers after a panic
        let orig_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            disable_raw_mode().unwrap();
            execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
            orig_hook(panic_info);
            std::process::exit(1);
        }));

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();
        terminal.clear().unwrap();

        Self {
            terminal,
            mode: Mode::Browser,
            queue: Queue::new(toml.volume()),
            browser: Browser::new(),
            options: Options::new(toml),
            search: Search::new(),
            db: Database::default(),
        }
    }
    fn on_update(&mut self) {
        if self.db.needs_update() {
            self.browser.refresh();
            self.search.update_engine();

            self.db.stop();
        }

        if self.search.has_query_changed() {
            self.search.update_search();
        }

        self.queue.update();
    }
    pub fn run(&mut self, mut toml: Toml) -> std::io::Result<()> {
        let mut last_tick = Instant::now();

        self.db.sync_database(toml.paths());

        #[cfg(windows)]
        let tx = App::register_hotkeys(toml.clone());

        loop {
            if last_tick.elapsed() >= TICK_RATE {
                self.on_update();
                last_tick = Instant::now();
            }

            self.terminal.draw(|f| match self.mode {
                Mode::Browser => self.browser.draw(f, self.db.is_busy()),
                Mode::Queue => self.queue.draw(f),
                Mode::Options => self.options.draw(f, &toml),
                Mode::Search => self.search.draw(f),
            })?;

            #[cfg(windows)]
            if let Ok(recv) = tx.try_recv() {
                match recv {
                    HotkeyEvent::VolUp => {
                        let vol = self.queue.player.volume_up();
                        toml.set_volume(vol);
                    }
                    HotkeyEvent::VolDown => {
                        let vol = self.queue.player.volume_down();
                        toml.set_volume(vol);
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
                            modifiers: Modifier::from_u32(event.modifiers),
                        };

                        if toml.hotkey.quit.contains(&bind) {
                            break;
                        };

                        match event.code {
                            KeyCode::Char(c) if self.mode == Mode::Search => self.search.on_key(c),
                            KeyCode::Tab => {
                                self.mode = match self.mode {
                                    Mode::Browser | Mode::Options => Mode::Queue,
                                    Mode::Queue => Mode::Browser,
                                    Mode::Search => {
                                        self.search.on_tab();
                                        Mode::Queue
                                    }
                                };
                            }
                            KeyCode::Backspace => self.search.on_backspace(event.modifiers),
                            KeyCode::Enter => match self.mode {
                                Mode::Browser => {
                                    let songs = self.browser.on_enter();
                                    self.queue.player.add_songs(songs);
                                }
                                Mode::Queue => {
                                    if let Some(i) = self.queue.ui.index {
                                        self.queue.player.play_song(i);
                                    }
                                }
                                Mode::Search => self.search.on_enter(&mut self.queue.player),
                                Mode::Options => {
                                    self.options.on_enter(&mut self.queue.player, &mut toml)
                                }
                            },
                            KeyCode::Esc => match self.mode {
                                Mode::Search => self.search.on_escape(&mut self.mode),
                                Mode::Options => self.mode = Mode::Queue,
                                _ => (),
                            },
                            KeyCode::Char('1' | '!') => {
                                self.queue.move_constraint('1', event.modifiers);
                            }
                            KeyCode::Char('2' | '@') => {
                                self.queue.move_constraint('2', event.modifiers);
                            }
                            KeyCode::Char('3' | '#') => {
                                self.queue.move_constraint('3', event.modifiers);
                            }
                            _ if toml.hotkey.up.contains(&bind) => self.up(),
                            _ if toml.hotkey.down.contains(&bind) => self.down(),
                            _ if toml.hotkey.left.contains(&bind) => self.browser.prev(),
                            _ if toml.hotkey.right.contains(&bind) => self.browser.next(),
                            _ if toml.hotkey.play_pause.contains(&bind) => {
                                self.queue.player.toggle_playback()
                            }
                            _ if toml.hotkey.clear.contains(&bind) => self.queue.clear(),
                            _ if toml.hotkey.refresh_database.contains(&bind) => {
                                self.db.add_dirs(toml.paths());
                                self.db.sync_database(toml.paths());
                            }
                            _ if toml.hotkey.seek_backward.contains(&bind) => {
                                self.queue.player.seek_by(-SEEK_TIME)
                            }
                            _ if toml.hotkey.seek_forward.contains(&bind) => {
                                self.queue.player.seek_by(SEEK_TIME)
                            }
                            _ if toml.hotkey.previous.contains(&bind) => {
                                self.queue.player.prev_song()
                            }
                            _ if toml.hotkey.next.contains(&bind) => self.queue.player.next_song(),
                            _ if toml.hotkey.volume_up.contains(&bind) => {
                                let vol = self.queue.player.volume_up();
                                toml.set_volume(vol);
                            }
                            _ if toml.hotkey.volume_down.contains(&bind) => {
                                let vol = self.queue.player.volume_down();
                                toml.set_volume(vol);
                            }
                            _ if toml.hotkey.search.contains(&bind) => self.mode = Mode::Search,
                            _ if toml.hotkey.options.contains(&bind) => self.mode = Mode::Options,
                            _ if toml.hotkey.delete.contains(&bind) => {
                                if let Mode::Queue = self.mode {
                                    if let Some(i) = self.queue.ui.index {
                                        self.queue.player.delete_song(i);
                                    }
                                }
                            }
                            _ if toml.hotkey.random.contains(&bind) => {
                                self.queue.player.randomize()
                            }
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
        }
    }

    fn down(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.down(),
            Mode::Queue => self.queue.down(),
            Mode::Search => self.search.down(),
            Mode::Options => self.options.down(),
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

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use gonk_database::{Bind, Colors, Database, Hotkey, Key, Modifier, Toml};
use static_init::dynamic;
use std::io::{stdout, Stdout};
use std::time::Duration;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Instant,
};
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
static TOML: Toml = Toml::new().unwrap();
#[dynamic]
static COLORS: Colors = TOML.colors.clone();
#[dynamic]
static HK: Hotkey = TOML.hotkey.clone();

const TICK_RATE: Duration = Duration::from_millis(100);

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
    pub fn new() -> Self {
        //make sure the terminal recovers after a panic
        let orig_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
            disable_raw_mode().unwrap();
            terminal.show_cursor().unwrap();
            execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
            orig_hook(panic_info);
            std::process::exit(1);
        }));

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();
        terminal.clear().unwrap();
        terminal.hide_cursor().unwrap();

        Self {
            terminal,
            mode: Mode::Browser,
            queue: Queue::new(),
            browser: Browser::new(),
            options: Options::new(),
            search: Search::new(),
            db: Database::new().unwrap(),
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
    pub fn run(&mut self) -> std::io::Result<()> {
        let mut last_tick = Instant::now();
        let mut toml = Toml::new()?;

        #[cfg(windows)]
        let tx = App::register_hotkeys();

        self.db.sync_database(TOML.paths());

        loop {
            if last_tick.elapsed() >= TICK_RATE {
                self.on_update();
                last_tick = Instant::now();
            }

            #[cfg(windows)]
            self.handle_global_hotkeys(&tx, &mut toml);

            self.terminal.draw(|f| match self.mode {
                Mode::Browser => self.browser.draw(f, self.db.is_busy()),
                Mode::Queue => self.queue.draw(f),
                Mode::Options => self.options.draw(f),
                Mode::Search => self.search.draw(f),
            })?;

            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(event) => {
                        let bind = Bind {
                            key: Key::from(event.code),
                            modifiers: Modifier::from_u32(event.modifiers),
                        };

                        if HK.quit.contains(&bind) {
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
                                Mode::Options => self.options.on_enter(&mut self.queue.player),
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
                            _ if HK.up.contains(&bind) => self.up(),
                            _ if HK.down.contains(&bind) => self.down(),
                            _ if HK.left.contains(&bind) => self.browser.prev(),
                            _ if HK.right.contains(&bind) => self.browser.next(),
                            _ if HK.play_pause.contains(&bind) => {
                                self.queue.player.toggle_playback()
                            }
                            _ if HK.clear.contains(&bind) => self.queue.player.clear_songs(),
                            _ if HK.refresh_database.contains(&bind) => {
                                self.db.add_dirs(TOML.paths());
                                self.db.sync_database(TOML.paths());
                            }
                            _ if HK.seek_backward.contains(&bind) => self.queue.player.seek_bw(),
                            _ if HK.seek_forward.contains(&bind) => self.queue.player.seek_fw(),
                            _ if HK.previous.contains(&bind) => self.queue.player.prev_song(),
                            _ if HK.next.contains(&bind) => self.queue.player.next_song(),
                            _ if HK.volume_up.contains(&bind) => {
                                let vol = self.queue.player.volume_up();
                                toml.set_volume(vol);
                            }
                            _ if HK.volume_down.contains(&bind) => {
                                let vol = self.queue.player.volume_down();
                                toml.set_volume(vol);
                            }
                            _ if HK.search.contains(&bind) => self.mode = Mode::Search,
                            _ if HK.options.contains(&bind) => self.mode = Mode::Options,
                            _ if HK.delete.contains(&bind) => {
                                if let Mode::Queue = self.mode {
                                    if let Some(i) = self.queue.ui.index {
                                        self.queue.player.delete_song(i);
                                    }
                                }
                            }
                            _ if HK.random.contains(&bind) => self.queue.player.randomize(),
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
                    Event::Resize(..) => (),
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

    //TODO: this should be platform agnostic
    fn handle_global_hotkeys(&mut self, tx: &Receiver<HotkeyEvent>, toml: &mut Toml) {
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
    }

    #[cfg(windows)]
    fn register_hotkeys() -> Receiver<HotkeyEvent> {
        use win_hotkey::{keys, modifiers, Listener};

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

    #[cfg(unix)]
    fn register_hotkeys(&self) -> Receiver<HotkeyEvent> {
        todo!();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        self.terminal.show_cursor().unwrap();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
    }
}

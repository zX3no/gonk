use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use gonk_database::{Bind, Colors, Database, Hotkey, Key, Modifier, Toml};
use rodio::Player;
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
}

impl App {
    pub fn new() -> Self {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout())).unwrap();

        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
        enable_raw_mode().unwrap();
        terminal.clear().unwrap();
        terminal.hide_cursor().unwrap();

        Self {
            terminal,
            mode: Mode::Browser,
        }
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut last_tick = Instant::now();

        let db = Database::new().unwrap();
        db.add(&TOML.paths());

        let mut toml = Toml::new().unwrap();
        let mut queue = Queue::new();
        let mut browser = Browser::new();
        let mut options = Options::new();
        let mut player = Player::new(TOML.volume());

        let mut search = Search::new();
        search.update_engine();

        let tx = App::register_hotkeys();

        loop {
            App::global_hotkeys(&tx, &mut player, &mut toml);

            self.terminal.draw(|f| match self.mode {
                Mode::Browser => browser.draw(f),
                Mode::Queue => queue.draw(f, &player),
                Mode::Options => options.draw(f),
                Mode::Search => search.draw(f),
            })?;

            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            //on update
            if last_tick.elapsed() >= TICK_RATE {
                if let Some(busy) = db.is_busy() {
                    if busy {
                        browser.refresh();
                        search.update_engine();
                    }
                    browser.is_busy = busy;
                }

                if search.has_query_changed() {
                    search.update_search();
                }
                last_tick = Instant::now();
            }

            //skip to the next track when ready
            player.update();
            //sync the player and queue
            queue.update(&player);

            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(event) => {
                        let modifiers = event.modifiers;
                        let key = event.code;

                        let bind = Bind {
                            key: Key::from(key),
                            modifiers: Modifier::from_u32(modifiers),
                        };

                        if HK.quit.contains(&bind) {
                            break;
                        };

                        match key {
                            KeyCode::Char(c) if self.mode == Mode::Search => {
                                search.on_key(c);
                            }
                            KeyCode::Tab => {
                                self.mode = match self.mode {
                                    Mode::Browser | Mode::Options => Mode::Queue,
                                    Mode::Queue => Mode::Browser,
                                    Mode::Search => {
                                        search.on_tab();
                                        Mode::Queue
                                    }
                                };
                            }
                            KeyCode::Backspace => search.on_backspace(modifiers),
                            KeyCode::Enter => match self.mode {
                                Mode::Browser => {
                                    let songs = browser.on_enter();
                                    player.add_songs(songs);
                                }
                                Mode::Queue => {
                                    if let Some(i) = queue.selected() {
                                        player.play_song(i);
                                    }
                                }
                                Mode::Search => search.on_enter(&mut player),
                                Mode::Options => {
                                    if let Some(dir) = options.on_enter(&mut player) {
                                        db.delete_path(&dir);
                                        browser.refresh();
                                        search.update_engine();
                                    }
                                }
                            },
                            KeyCode::Esc => match self.mode {
                                Mode::Search => search.on_escape(&mut self.mode),
                                Mode::Options => self.mode = Mode::Queue,
                                _ => (),
                            },
                            KeyCode::Char('1' | '!') => {
                                queue.move_constraint('1', modifiers);
                            }
                            KeyCode::Char('2' | '@') => {
                                queue.move_constraint('2', modifiers);
                            }
                            KeyCode::Char('3' | '#') => {
                                queue.move_constraint('3', modifiers);
                            }

                            _ if HK.up.contains(&bind) => {
                                self.up(&mut browser, &mut queue, &mut search, &mut options);
                            }
                            _ if HK.down.contains(&bind) => {
                                self.down(&mut browser, &mut queue, &mut search, &mut options);
                            }
                            _ if HK.left.contains(&bind) => browser.prev(),
                            _ if HK.right.contains(&bind) => browser.next(),
                            _ if HK.play_pause.contains(&bind) => player.toggle_playback(),
                            _ if HK.clear.contains(&bind) => player.clear_songs(),
                            _ if HK.refresh_database.contains(&bind) => {
                                for path in TOML.paths() {
                                    db.force_add(&path);
                                }
                            }
                            _ if HK.seek_backward.contains(&bind) => player.seek_bw(),
                            _ if HK.seek_forward.contains(&bind) => player.seek_fw(),
                            _ if HK.previous.contains(&bind) => player.prev_song(),
                            _ if HK.next.contains(&bind) => player.next_song(),
                            _ if HK.volume_up.contains(&bind) => {
                                let vol = player.volume_up();
                                toml.set_volume(vol);
                            }
                            _ if HK.volume_down.contains(&bind) => {
                                let vol = player.volume_down();
                                toml.set_volume(vol);
                            }
                            _ if HK.search.contains(&bind) => self.mode = Mode::Search,
                            _ if HK.options.contains(&bind) => self.mode = Mode::Options,
                            _ if HK.delete.contains(&bind) => {
                                if let Mode::Queue = self.mode {
                                    if let Some(i) = queue.selected() {
                                        player.delete_song(i);
                                    }
                                }
                            }
                            _ if HK.random.contains(&bind) => player.randomize(),
                            _ => (),
                        }
                    }
                    Event::Mouse(event) => match event.kind {
                        MouseEventKind::ScrollUp => {
                            self.up(&mut browser, &mut queue, &mut search, &mut options);
                        }
                        MouseEventKind::ScrollDown => {
                            self.down(&mut browser, &mut queue, &mut search, &mut options);
                        }
                        MouseEventKind::Down(_) => {
                            queue.clicked_pos = Some((event.column, event.row));
                        }
                        _ => (),
                    },
                    Event::Resize(..) => (),
                }
            }
        }

        Ok(())
    }

    fn up(
        &self,
        browser: &mut Browser,
        queue: &mut Queue,
        search: &mut Search,
        options: &mut Options,
    ) {
        match self.mode {
            Mode::Browser => browser.up(),
            Mode::Queue => queue.up(),
            Mode::Search => search.up(),
            Mode::Options => options.up(),
        }
    }

    fn down(
        &self,
        browser: &mut Browser,
        queue: &mut Queue,
        search: &mut Search,
        options: &mut Options,
    ) {
        match self.mode {
            Mode::Browser => browser.down(),
            Mode::Queue => queue.down(),
            Mode::Search => search.down(),
            Mode::Options => options.down(),
        }
    }

    #[cfg(windows)]
    fn global_hotkeys(tx: &Receiver<HotkeyEvent>, player: &mut Player, toml: &mut Toml) {
        if let Ok(recv) = tx.try_recv() {
            match recv {
                HotkeyEvent::VolUp => {
                    let vol = player.volume_up();
                    toml.set_volume(vol);
                }
                HotkeyEvent::VolDown => {
                    let vol = player.volume_down();
                    toml.set_volume(vol);
                }
                HotkeyEvent::PlayPause => player.toggle_playback(),
                HotkeyEvent::Prev => player.prev_song(),
                HotkeyEvent::Next => player.next_song(),
            }
        }
    }

    #[cfg(unix)]
    fn global_hotkeys(tx: &Receiver<HotkeyEvent>, player: &mut Player, toml: &mut Toml) {}

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
    fn register_hotkeys(&self) -> Receiver<HotkeyEvent> {}
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

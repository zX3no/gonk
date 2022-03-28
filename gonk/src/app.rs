use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use gonk_database::{Bind, Database, Key, Modifier, Toml};
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

//TODO: index trait for ui

// pub trait Indexable {
//     fn up(&mut self);
//     fn down(&mut self);
// }

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
        let tick_rate = Duration::from_millis(100);

        let db = Database::new().unwrap();
        let toml = Toml::new().unwrap();
        let mut queue = Queue::new(toml.volume());
        let mut browser = Browser::new(&db);
        let mut search = Search::new(&db);
        db.add(&toml.paths());
        let mut options = Options::new(toml);
        let hk = options.hotkeys().clone();
        let tx = self.register_hotkeys();

        loop {
            #[cfg(windows)]
            if let Ok(recv) = tx.try_recv() {
                match recv {
                    HotkeyEvent::VolUp => queue.volume_up(),
                    HotkeyEvent::VolDown => queue.volume_down(),
                    HotkeyEvent::PlayPause => queue.play_pause(),
                    HotkeyEvent::Prev => queue.prev(),
                    HotkeyEvent::Next => queue.next(),
                }
            }

            let colors = options.colors();

            self.terminal.draw(|f| match self.mode {
                Mode::Browser => browser.draw(f),
                Mode::Queue => queue.draw(f, colors),
                Mode::Options => options.draw(f),
                Mode::Search => search.draw(f, colors),
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            //on update
            if last_tick.elapsed() >= tick_rate {
                queue.on_update();

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

            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(event) => {
                        let modifiers = event.modifiers;
                        let key = event.code;

                        let bind = Bind {
                            key: Key::from(key),
                            modifiers: Modifier::from_u32(modifiers),
                        };

                        //exit
                        if hk.quit.contains(&bind) {
                            break;
                        };

                        match key {
                            KeyCode::Char(c) if self.mode == Mode::Search => {
                                search.on_key(c);
                            }
                            KeyCode::Tab => {
                                self.mode = match self.mode {
                                    Mode::Browser => Mode::Queue,
                                    Mode::Queue => Mode::Browser,
                                    Mode::Search => {
                                        search.on_tab();
                                        Mode::Queue
                                    }
                                    Mode::Options => Mode::Queue,
                                };
                            }
                            KeyCode::Backspace => search.on_backspace(modifiers),
                            KeyCode::Enter => match self.mode {
                                Mode::Browser => {
                                    let songs = browser.on_enter();
                                    queue.add(songs);
                                }
                                Mode::Queue => queue.select(),
                                Mode::Search => search.on_enter(&mut queue),
                                Mode::Options => {
                                    if let Some(dir) = options.on_enter(&mut queue) {
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
                            KeyCode::Char('1') | KeyCode::Char('!') => {
                                queue.move_constraint('1', modifiers)
                            }
                            KeyCode::Char('2') | KeyCode::Char('@') => {
                                queue.move_constraint('2', modifiers)
                            }
                            KeyCode::Char('3') | KeyCode::Char('#') => {
                                queue.move_constraint('3', modifiers)
                            }
                            _ if hk.up.contains(&bind) => {
                                self.up(&mut browser, &mut queue, &mut search, &mut options)
                            }
                            _ if hk.down.contains(&bind) => {
                                self.down(&mut browser, &mut queue, &mut search, &mut options)
                            }
                            _ if hk.left.contains(&bind) => browser.prev(),
                            _ if hk.right.contains(&bind) => browser.next(),
                            _ if hk.play_pause.contains(&bind) => queue.play_pause(),
                            _ if hk.clear.contains(&bind) => queue.clear(),
                            _ if hk.refresh_database.contains(&bind) => {
                                for path in options.paths() {
                                    db.force_add(path);
                                }
                            }
                            _ if hk.seek_backward.contains(&bind) => queue.seek_bw(),
                            _ if hk.seek_forward.contains(&bind) => queue.seek_fw(),
                            _ if hk.previous.contains(&bind) => queue.prev(),
                            _ if hk.next.contains(&bind) => queue.next(),
                            _ if hk.volume_up.contains(&bind) => {
                                queue.volume_up();
                                options.save_volume(queue.player.volume());
                            }
                            _ if hk.volume_down.contains(&bind) => {
                                queue.volume_down();
                                options.save_volume(queue.player.volume());
                            }
                            _ if hk.search.contains(&bind) => self.mode = Mode::Search,
                            _ if hk.options.contains(&bind) => self.mode = Mode::Options,
                            _ if hk.delete.contains(&bind) => {
                                if let Mode::Queue = self.mode {
                                    queue.delete_selected();
                                }
                            }
                            _ if hk.random.contains(&bind) => queue.randomize(),
                            _ => (),
                        }
                    }
                    Event::Mouse(event) => match event.kind {
                        MouseEventKind::ScrollUp => {
                            self.up(&mut browser, &mut queue, &mut search, &mut options)
                        }
                        MouseEventKind::ScrollDown => {
                            self.down(&mut browser, &mut queue, &mut search, &mut options)
                        }
                        MouseEventKind::Down(_) => {
                            queue.clicked_pos = Some((event.column, event.row));
                        }
                        _ => (),
                    },
                    _ => (),
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
    fn register_hotkeys(&self) -> Receiver<HotkeyEvent> {
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
    // fn register_hotkeys(&self) -> Receiver<HotkeyEvent> {
    fn register_hotkeys(&self) {}
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

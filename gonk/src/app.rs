use crate::HotkeyEvent;
use crossterm::event::{self, Event, KeyCode, MouseEventKind};
use gonk_database::{Bind, Database, Key, Modifier, Toml};
use std::time::Duration;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Instant,
};
use tui::{backend::Backend, Terminal};

use self::new_search::NewSearch;
use {browser::Browser, options::Options, queue::Queue, search::Search};

mod browser;
mod new_search;
mod options;
mod queue;
mod search;

#[derive(PartialEq, Debug)]
pub enum Mode {
    Browser,
    Queue,
    Search,
    Options,
}

pub struct App {
    pub mode: Mode,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Browser,
        }
    }
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(16);
        let tx = self.register_hotkeys();

        let db = Database::new().unwrap();
        let toml = Toml::new().unwrap();
        let mut queue = Queue::new(toml.volume());
        let mut browser = Browser::new(&db);
        let new_search = NewSearch::default();
        let mut search = Search::new(&db);
        let mut options = Options::new(toml);
        let hk = options.hotkeys().clone();

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

            terminal.draw(|f| match self.mode {
                Mode::Browser => browser.draw(f),
                Mode::Queue => queue.draw(f, colors),
                Mode::Options => options.draw(f),
                // Mode::Search => search.draw(f, &db, colors),
                Mode::Search => new_search.draw(f, colors),
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

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
                                Mode::Search => {
                                    if let Some(songs) = search.get_songs() {
                                        queue.add(songs);
                                    }
                                }
                                Mode::Options => {
                                    if let Some(dir) = options.on_enter(&mut queue) {
                                        db.delete_path(&dir);
                                        browser.refresh();
                                        search.refresh();
                                    }
                                }
                            },
                            KeyCode::Esc => {
                                match self.mode {
                                    Mode::Search => {
                                        if search.on_escape() {
                                            //TODO previous mode should be stored
                                            //self.mod = self.mod.last()
                                            self.mode = Mode::Queue;
                                        }
                                    }
                                    Mode::Options => {
                                        self.mode = Mode::Queue;
                                    }
                                    _ => (),
                                }
                            }
                            KeyCode::Char('1') | KeyCode::Char('!') => {
                                queue.move_constraint('1', modifiers)
                            }
                            KeyCode::Char('2') | KeyCode::Char('@') => {
                                queue.move_constraint('2', modifiers)
                            }
                            KeyCode::Char('3') | KeyCode::Char('#') => {
                                queue.move_constraint('3', modifiers)
                            }
                            _ if hk.up.contains(&bind) => match self.mode {
                                Mode::Browser => browser.up(),
                                Mode::Queue => queue.up(),
                                Mode::Search => search.up(),
                                Mode::Options => options.up(),
                            },
                            _ if hk.down.contains(&bind) => match self.mode {
                                Mode::Browser => browser.down(),
                                Mode::Queue => queue.down(),
                                Mode::Search => search.down(),
                                Mode::Options => options.down(),
                            },
                            _ if hk.left.contains(&bind) => browser.prev(),
                            _ if hk.right.contains(&bind) => browser.next(),
                            _ if hk.play_pause.contains(&bind) => queue.play_pause(),
                            _ if hk.clear.contains(&bind) => queue.clear(),
                            _ if hk.refresh_database.contains(&bind) => {
                                //update all the music in the db
                                let paths = options.paths();
                                paths.iter().for_each(|path| db.add(path));
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
                        MouseEventKind::ScrollUp => match self.mode {
                            Mode::Browser => browser.up(),
                            Mode::Queue => queue.up(),
                            Mode::Search => search.up(),
                            Mode::Options => options.up(),
                        },
                        MouseEventKind::ScrollDown => match self.mode {
                            Mode::Browser => browser.down(),
                            Mode::Queue => queue.down(),
                            Mode::Search => search.down(),
                            Mode::Options => options.down(),
                        },
                        MouseEventKind::Down(_) => {
                            queue.clicked_pos = Some((event.column, event.row));
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }

            if last_tick.elapsed() >= tick_rate {
                queue.update();

                if let Some(busy) = db.is_busy() {
                    if busy {
                        browser.refresh();
                        search.refresh();
                    }
                    browser.is_busy = busy;
                }

                if search.has_query_changed() {
                    search.update_search();
                }
                last_tick = Instant::now();
            }
        }

        Ok(())
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
    fn register_hotkeys(&self) -> Receiver<HotkeyEvent> {
        todo!();
    }
}

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use gonk_core::{Bind, ClientConfig, Key, Modifier};
use gonk_server::Client;
use static_init::dynamic;
use std::cell::RefCell;
use std::io::{stdout, Stdout};
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use {browser::Browser, queue::Queue};

mod browser;
mod queue;

#[derive(PartialEq, Debug)]
pub enum Mode {
    Browser,
    Queue,
    //TODO: search and options
    //need to be re-implimented
}

#[dynamic]
static CONFIG: ClientConfig = ClientConfig::new();

const TICK_RATE: Duration = Duration::from_millis(10);
const POLL_RATE: Duration = Duration::from_millis(4);
const SEEK_TIME: f64 = 10.0;

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub mode: Mode,
    queue: Queue,
    browser: Browser,
    client: Rc<RefCell<Client>>,
}

impl App {
    pub fn new(client: Client) -> Self {
        optick::event!("new app");
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

        let client = Rc::new(RefCell::new(client));

        Self {
            terminal,
            mode: Mode::Browser,
            queue: Queue::new(client.clone()),
            browser: Browser::new(client.clone()),
            client,
        }
    }
    pub fn run(&mut self) -> std::io::Result<()> {
        let mut last_tick = Instant::now();

        loop {
            optick::event!("loop");

            if last_tick.elapsed() >= TICK_RATE {
                self.queue.update();
                last_tick = Instant::now();
            }

            self.terminal.draw(|f| match self.mode {
                Mode::Browser => self.browser.draw(f),
                Mode::Queue => self.queue.draw(f),
            })?;

            self.client.borrow_mut().update();

            if crossterm::event::poll(POLL_RATE)? {
                match event::read()? {
                    Event::Key(event) => {
                        optick::event!("key event");
                        let bind = Bind {
                            key: Key::from(event.code),
                            modifiers: Modifier::from_u32(event.modifiers),
                        };

                        if CONFIG.hotkey.quit.contains(&bind) {
                            break;
                        };

                        match event.code {
                            KeyCode::Tab => {
                                self.mode = match self.mode {
                                    Mode::Browser => Mode::Queue,
                                    Mode::Queue => Mode::Browser,
                                };
                            }
                            KeyCode::Backspace => (),
                            //TODO: we should not send songs over tcp it should be ids only
                            KeyCode::Enter => match self.mode {
                                Mode::Browser => {
                                    self.browser.on_enter();
                                    // let songs: Vec<u64> = self
                                    //     .browser
                                    //     .on_enter()
                                    //     .iter()
                                    //     .flat_map(|song| song.id)
                                    //     .collect();
                                    //self.queue().add_ids(&songs);
                                }
                                Mode::Queue => {
                                    if let Some(i) = self.queue.ui.index {
                                        self.client.borrow_mut().play_index(i);
                                    }
                                }
                            },
                            KeyCode::Esc => (),
                            KeyCode::Char('1' | '!') => {
                                self.queue.move_constraint('1', event.modifiers);
                            }
                            KeyCode::Char('2' | '@') => {
                                self.queue.move_constraint('2', event.modifiers);
                            }
                            KeyCode::Char('3' | '#') => {
                                self.queue.move_constraint('3', event.modifiers);
                            }
                            _ if CONFIG.hotkey.up.contains(&bind) => self.up(),
                            _ if CONFIG.hotkey.down.contains(&bind) => self.down(),
                            _ if CONFIG.hotkey.left.contains(&bind) => self.browser.prev(),
                            _ if CONFIG.hotkey.right.contains(&bind) => self.browser.next(),
                            _ if CONFIG.hotkey.play_pause.contains(&bind) => {
                                self.client.borrow_mut().toggle_playback()
                            }
                            _ if CONFIG.hotkey.clear.contains(&bind) => self.queue.clear(),
                            _ if CONFIG.hotkey.refresh_database.contains(&bind) => {
                                todo!();
                            }
                            _ if CONFIG.hotkey.seek_backward.contains(&bind) => {
                                self.client.borrow_mut().seek_by(-SEEK_TIME)
                            }
                            _ if CONFIG.hotkey.seek_forward.contains(&bind) => {
                                self.client.borrow_mut().seek_by(SEEK_TIME)
                            }
                            _ if CONFIG.hotkey.previous.contains(&bind) => {
                                self.client.borrow_mut().prev()
                            }
                            _ if CONFIG.hotkey.next.contains(&bind) => {
                                self.client.borrow_mut().next()
                            }
                            _ if CONFIG.hotkey.volume_up.contains(&bind) => {
                                self.client.borrow_mut().volume_up()
                            }
                            _ if CONFIG.hotkey.volume_down.contains(&bind) => {
                                self.client.borrow_mut().volume_down();
                            }
                            _ if CONFIG.hotkey.search.contains(&bind) => (),
                            _ if CONFIG.hotkey.options.contains(&bind) => (),
                            _ if CONFIG.hotkey.delete.contains(&bind) => {
                                if let Mode::Queue = self.mode {
                                    if let Some(i) = self.queue.ui.index {
                                        self.client.borrow_mut().delete_song(i);
                                    }
                                }
                            }
                            _ if CONFIG.hotkey.random.contains(&bind) => {
                                self.client.borrow_mut().randomize()
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
        }
    }

    fn down(&mut self) {
        match self.mode {
            Mode::Browser => self.browser.down(),
            Mode::Queue => self.queue.down(),
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

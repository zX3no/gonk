use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::stdout;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, ListItem, ListState, Paragraph, Wrap};
use tui::Terminal;

use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use crate::musiclibrary::MusicLibrary;

type Result = crossterm::Result<()>;

enum Panel {
    Browser,
    Queue,
}

pub struct App {
    ml: MusicLibrary,
    browser_state: ListState,
    queue_state: ListState,
    quit: bool,
    search_mode: bool,
    query: String,
    selected_panel: Panel,
}
impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();

        Self {
            ml: MusicLibrary::new(),
            browser_state: ListState::default(),
            queue_state: ListState::default(),
            quit: false,
            search_mode: false,
            query: String::new(),
            selected_panel: Panel::Browser,
        }
    }
    pub fn run(&mut self) -> Result {
        let mut terminal =
            Terminal::new(CrosstermBackend::new(stdout())).expect("couldn't create terminal");

        terminal.clear().unwrap();
        terminal.hide_cursor().unwrap();

        loop {
            terminal
                .draw(|f| {
                    let size = f.size();

                    match self.selected_panel {
                        Panel::Browser => {
                            self.browser_state.select(self.ml.browser_selection());
                        }
                        Panel::Queue => {
                            self.queue_state.select(self.ml.queue_selection());
                        }
                    }

                    let list = self.ml.items();

                    let list: Vec<ListItem> = list
                        .iter()
                        .map(|item| ListItem::new(item.clone()))
                        .collect();

                    let queue: Vec<ListItem> = self
                        .ml
                        .queue()
                        .iter()
                        .map(|item| ListItem::new(item.clone()))
                        .collect();

                    let queue = tui::widgets::List::new(queue)
                        .block(Block::default().title("Queue").borders(Borders::ALL))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                        .highlight_symbol(">>");

                    let browser = tui::widgets::List::new(list)
                        .block(
                            Block::default()
                                .title(self.ml.title())
                                .borders(Borders::ALL),
                        )
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                        .highlight_symbol(">>");

                    let left = if self.search_mode {
                        Rect::new(0, 0, size.width / 3, size.height - 3)
                    } else {
                        Rect::new(0, 0, size.width / 3, size.height)
                    };

                    let bottom_left = Rect::new(0, size.height - 3, size.width / 3, 3);
                    let b = Block::default()
                        .title(self.ml.volume())
                        .borders(Borders::ALL);

                    let g = Gauge::default()
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(self.ml.now_playing()),
                        )
                        .gauge_style(
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Black)
                                .add_modifier(Modifier::ITALIC),
                        )
                        .percent(self.ml.progress_percent())
                        .label(self.ml.progress());

                    let right = Rect::new(
                        size.width / 3,
                        0,
                        size.width - size.width / 3,
                        size.height - 3,
                    );
                    let bottom_right = Rect::new(
                        size.width / 3,
                        size.height - 3,
                        size.width - size.width / 3,
                        3,
                    );

                    let search = Paragraph::new(self.query.clone())
                        .block(Block::default().title("Search").borders(Borders::ALL))
                        .style(Style::default())
                        .wrap(Wrap { trim: true });

                    f.render_stateful_widget(browser, left, &mut self.browser_state);
                    f.render_stateful_widget(queue, right, &mut self.queue_state);
                    // f.render_widget(b, right);
                    f.render_widget(g, bottom_right);
                    if self.search_mode {
                        f.render_widget(search, bottom_left);
                    }
                })
                .unwrap();

            if self.search_mode {
                self.search()?;
            } else {
                self.handle_input()?;
            }

            if self.quit {
                break;
            }
        }

        execute!(stdout(), LeaveAlternateScreen).unwrap();
        Ok(())
    }

    pub fn search(&mut self) -> Result {
        if poll(Duration::from_millis(10))? {
            if let Event::Key(KeyEvent { code, modifiers }) = read()? {
                if modifiers == KeyModifiers::CONTROL {
                    match code {
                        KeyCode::Backspace => {
                            self.query = String::new();
                            self.ml.reset_filter();
                        }
                        KeyCode::Char('w') => {
                            self.query = String::new();
                            self.ml.reset_filter();
                        }
                        _ => (),
                    }
                } else {
                    match code {
                        KeyCode::Backspace => {
                            self.query.pop();
                            self.ml.reset_filter();
                            self.ml.filter(&self.query);
                        }
                        KeyCode::Esc => {
                            self.search_mode = false;
                            self.query = String::new();
                            self.ml.reset_filter();
                        }
                        KeyCode::Enter => {
                            self.search_mode = false;
                            self.query = String::new();

                            //if search is empty reset results
                            if self.ml.filter_len() == 0 {
                                self.ml.reset_filter();
                            }
                        }
                        KeyCode::Char(c) => {
                            self.query.push(c);
                            self.ml.filter(&self.query);
                        }

                        _ => (),
                    }
                }
            }
        }
        Ok(())
    }
    pub fn handle_input(&mut self) -> Result {
        if poll(Duration::from_millis(16))? {
            if let Event::Key(KeyEvent { code, modifiers }) = read()? {
                match modifiers {
                    KeyModifiers::CONTROL => match code {
                        KeyCode::Char('c') => {
                            self.quit = true;
                        }
                        _ => (),
                    },
                    KeyModifiers::SHIFT => match code {
                        KeyCode::Char('L') => {
                            self.browser_state.select(None);
                            self.selected_panel = Panel::Queue;
                        }
                        KeyCode::Char('H') => {
                            self.queue_state.select(None);
                            self.selected_panel = Panel::Browser;
                        }
                        _ => (),
                    },
                    _ => {
                        match code {
                            //if you type a number then press enter it should go to that track

                            // KeyCode::Char('1')
                            // | KeyCode::Char('2')
                            // | KeyCode::Char('3')
                            // | KeyCode::Char('4')
                            // | KeyCode::Char('5')
                            // | KeyCode::Char('6')
                            // | KeyCode::Char('7')
                            // | KeyCode::Char('8')
                            // | KeyCode::Char('9')
                            // | KeyCode::Char('0') => todo!(),
                            KeyCode::Char('c') => self.quit = true,
                            KeyCode::Down | KeyCode::Char('j') => match self.selected_panel {
                                Panel::Browser => {
                                    self.ml.down();
                                }
                                Panel::Queue => {
                                    self.ml.queue_down();
                                }
                            },
                            KeyCode::Up | KeyCode::Char('k') => match self.selected_panel {
                                Panel::Browser => {
                                    self.ml.up();
                                }
                                Panel::Queue => {
                                    self.ml.queue_up();
                                }
                            },
                            KeyCode::Enter => match self.selected_panel {
                                Panel::Browser => self.ml.add_to_queue(),
                                //todo remove placeholder
                                Panel::Queue => self.ml.next(),
                            },
                            KeyCode::Char('l') => {
                                if let Panel::Browser = self.selected_panel {
                                    self.ml.next_mode()
                                }
                            }
                            KeyCode::Backspace | KeyCode::Char('h') => {
                                if let Panel::Browser = self.selected_panel {
                                    self.ml.prev_mode()
                                }
                            }
                            KeyCode::Esc => self.ml.reset_filter(),
                            KeyCode::Char(' ') => self.ml.toggle_playback(),
                            KeyCode::Char('-') => self.ml.decrease_volume(),
                            KeyCode::Char('=') => self.ml.increase_volume(),
                            KeyCode::Char('/') => {
                                self.search_mode = true;
                                //reset the view everytime we enter search mode
                                self.ml.reset_filter();
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

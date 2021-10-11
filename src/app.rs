use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::stdout;
use std::panic::panic_any;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, ListItem, ListState};
use tui::Terminal;

use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use crate::musiclibrary::MusicLibrary;

type Result = crossterm::Result<()>;

pub struct App {
    ml: MusicLibrary,
    state: ListState,
    quit: bool,
    search_mode: bool,
    query: String,
}
impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();

        Self {
            ml: MusicLibrary::new(),
            state: ListState::default(),
            quit: false,
            search_mode: false,
            query: String::new(),
        }
    }
    pub fn run(&mut self) -> Result {
        let mut terminal =
            Terminal::new(CrosstermBackend::new(stdout())).expect("couldn't create terminal");

        terminal.clear().unwrap();

        loop {
            terminal
                .draw(|f| {
                    let size = f.size();

                    self.state.select(self.ml.selection());

                    let list = self.ml.items();

                    let list: Vec<ListItem> = list
                        .iter()
                        .map(|item| ListItem::new(item.clone()))
                        .collect();

                    let l = tui::widgets::List::new(list)
                        .block(
                            Block::default()
                                .title(self.ml.title())
                                .borders(Borders::ALL),
                        )
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                        .highlight_symbol(">>");

                    let left = Rect::new(0, 0, size.width / 3, size.height);

                    let b = Block::default().title("Block").borders(Borders::ALL);
                    let right =
                        Rect::new(size.width / 3, 0, size.width - size.width / 3, size.height);

                    f.render_stateful_widget(l, left, &mut self.state);
                    f.render_widget(b, right);
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
        if poll(Duration::from_millis(16))? {
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
                        KeyCode::Esc | KeyCode::Enter => {
                            self.search_mode = false;
                            self.query = String::new();
                            self.ml.reset_filter();
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
            //TODO wtf is this?
            match read()? {
                Event::Key(event) => match event {
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                    } => self.quit = true,
                    KeyEvent {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.down(),
                    KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.up(),
                    KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.next_mode(),
                    KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('h'),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.prev_mode(),
                    KeyEvent {
                        code: KeyCode::Char(' '),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.player.toggle_playback(),
                    KeyEvent {
                        code: KeyCode::Char('-'),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.player.decrease_volume(),
                    KeyEvent {
                        code: KeyCode::Char('='),
                        modifiers: KeyModifiers::NONE,
                    } => self.ml.player.increase_volume(),
                    KeyEvent {
                        code: KeyCode::Char('/'),
                        modifiers: KeyModifiers::NONE,
                    } => self.search_mode = true,
                    _ => (),
                },
                Event::Mouse(_) => (),
                Event::Resize(_, _) => (),
            }
        }
        Ok(())
    }
}

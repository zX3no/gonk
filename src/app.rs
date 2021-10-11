use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::stdout;
use std::panic::panic_any;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, ListItem, ListState, Paragraph, Wrap};
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

                    let left = if self.search_mode {
                        Rect::new(0, 0, size.width / 3, size.height - 3)
                    } else {
                        Rect::new(0, 0, size.width / 3, size.height)
                    };

                    let bottom_left = Rect::new(0, size.height - 3, size.width / 3, 3);
                    let b = Block::default()
                        .title(self.ml.player.volume.to_string())
                        .borders(Borders::ALL);
                    let g = Gauge::default()
                        .block(Block::default().borders(Borders::ALL))
                        .gauge_style(
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Black)
                                .add_modifier(Modifier::ITALIC),
                        )
                        .percent(15)
                        .label("0:35/2:59");
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
                        .style(Style::default().fg(Color::White).bg(Color::Black))
                        .wrap(Wrap { trim: true });

                    f.render_stateful_widget(l, left, &mut self.state);
                    if self.search_mode {
                        f.render_widget(search, bottom_left);
                    }
                    f.render_widget(b, right);
                    f.render_widget(g, bottom_right);
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
                        KeyCode::Esc => {
                            self.search_mode = false;
                            self.query = String::new();
                            self.ml.reset_filter();
                        }
                        KeyCode::Enter => {
                            self.search_mode = false;
                            self.query = String::new();
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
                if modifiers == KeyModifiers::CONTROL {
                    match code {
                        KeyCode::Char('c') => {
                            self.quit = true;
                        }
                        _ => (),
                    }
                } else {
                    match code {
                        KeyCode::Char('c') => self.quit = true,
                        KeyCode::Down | KeyCode::Char('j') => self.ml.down(),
                        KeyCode::Up | KeyCode::Char('k') => self.ml.up(),
                        KeyCode::Enter | KeyCode::Char('l') => self.ml.next_mode(),
                        KeyCode::Backspace | KeyCode::Char('h') => self.ml.prev_mode(),
                        KeyCode::Esc => self.ml.reset_filter(),
                        KeyCode::Char(' ') => self.ml.player.toggle_playback(),
                        KeyCode::Char('-') => self.ml.player.decrease_volume(),
                        KeyCode::Char('=') => self.ml.player.increase_volume(),
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
        Ok(())
    }
}

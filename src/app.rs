use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::stdout;
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
}
impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();

        Self {
            ml: MusicLibrary::new(),
            state: ListState::default(),
            quit: false,
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

            self.handle_input()?;

            if self.quit {
                break;
            }
        }

        Ok(())
    }

    pub fn handle_input(&mut self) -> Result {
        if poll(Duration::from_millis(100))? {
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
                    _ => (),
                },
                Event::Mouse(_) => (),
                Event::Resize(_, _) => (),
            }
        }
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        execute!(stdout(), LeaveAlternateScreen).unwrap();
    }
}

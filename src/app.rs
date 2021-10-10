use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io::stdout;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, BorderType, Borders, List, ListItem, ListState};
use tui::Terminal;

use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use crate::database::Artist;
use crate::index::get_artists;

type Result = crossterm::Result<()>;

pub struct App {
    selected: usize,
    quit: bool,
}
impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();
        Self {
            selected: 0,
            quit: false,
        }
    }
    pub fn run(&mut self) -> Result {
        let mut terminal =
            Terminal::new(CrosstermBackend::new(stdout())).expect("couldn't create terminal");

        terminal.clear().unwrap();
        let artists: Vec<ListItem> = get_artists()
            .iter()
            .map(|(_, v)| ListItem::new(v.name.clone()))
            .collect();
        let mut state = ListState::default();
        loop {
            terminal
                .draw(|f| {
                    let size = f.size();
                    let b = Block::default().title("Block").borders(Borders::ALL);

                    let l = List::new(artists.clone())
                        .block(Block::default().title("Artists").borders(Borders::ALL))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                        .highlight_symbol(">>");

                    let left = Rect::new(0, 0, size.width / 3, size.height);
                    let right =
                        Rect::new(size.width / 3, 0, size.width - size.width / 3, size.height);
                    state.select(Some(self.selected));
                    f.render_stateful_widget(l, left, &mut state);
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
                        code: KeyCode::Tab,
                        modifiers: KeyModifiers::NONE,
                    } => (),
                    KeyEvent {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::NONE,
                    } => self.selected += 1,
                    KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                    } => {
                        if self.selected != 0 {
                            self.selected -= 1;
                        }
                    }
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

use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::io::stdout;
use std::thread;
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
use crate::player::Player;

type Result = crossterm::Result<()>;

#[derive(PartialEq)]
enum Mode {
    Artist,
    Album,
    Track,
}

pub struct App<'a> {
    music: HashMap<String, Artist>,
    list: Vec<ListItem<'a>>,
    mode: Mode,
    album: String,
    artist: String,
    //this is very dumb
    selected: usize,
    list_size: usize,
    quit: bool,
}
impl<'a> App<'a> {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();
        let music = get_artists();

        let list: Vec<ListItem> = music
            .iter()
            .map(|(_, v)| ListItem::new(v.name.clone()))
            .collect();

        let list_size = list.clone().len() - 1;

        Self {
            music,
            list,
            mode: Mode::Artist,
            album: String::new(),
            artist: String::new(),
            selected: 0,
            list_size,
            quit: false,
        }
    }
    pub fn run(&mut self) -> Result {
        let mut terminal =
            Terminal::new(CrosstermBackend::new(stdout())).expect("couldn't create terminal");

        terminal.clear().unwrap();

        let mut state = ListState::default();
        loop {
            terminal
                .draw(|f| {
                    let size = f.size();
                    let b = Block::default().title("Block").borders(Borders::ALL);

                    let l = List::new(self.list.clone())
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
    pub fn play(&mut self) {
        let path = &self
            .music
            .get(&self.artist)
            .unwrap()
            .album(&self.album)
            .unwrap()
            .track(self.selected as u16 + 1)
            .unwrap()
            .path;

        let p = path.clone();

        thread::spawn(move || {
            Player::play(&p);
        });
    }
    pub fn change_mode(&mut self) {
        match self.mode {
            Mode::Artist => self.mode = Mode::Album,
            Mode::Album => self.mode = Mode::Track,
            Mode::Track => {
                self.play();
                return;
            }
        }
        self.update_mode();
        self.selected = 0;
    }
    pub fn update_mode(&mut self) {
        match self.mode {
            Mode::Artist => {
                //get artists
                let list: Vec<ListItem> = self
                    .music
                    .iter()
                    .map(|(_, v)| ListItem::new(v.name.clone()))
                    .collect();
                self.list_size = list.len() - 1;
                self.list = list;

                self.artist = String::new();
                self.album = String::new();
            }
            Mode::Album => {
                //get album from artist
                if !self.artist.is_empty() {
                    let list: Vec<ListItem> = self
                        .music
                        .get(&self.artist)
                        .unwrap()
                        .albums
                        .iter()
                        .map(|album| ListItem::new(album.title.clone()))
                        .collect();
                    self.list_size = list.len() - 1;
                    self.list = list;
                } else {
                    let mut i = 0;
                    for (_, v) in &self.music {
                        if i == self.selected {
                            self.artist = v.name.clone();
                            let list: Vec<ListItem> = v
                                .albums
                                .iter()
                                .map(|album| ListItem::new(album.title.clone()))
                                .collect();
                            self.list_size = list.len() - 1;
                            self.list = list;
                        }
                        i += 1;
                    }
                }
            }
            Mode::Track => {
                //get tracks from album
                let album = self
                    .music
                    .get(&self.artist)
                    .unwrap()
                    .albums
                    .get(self.selected)
                    .unwrap();

                self.album = album.title.clone();
                let list: Vec<ListItem> = album
                    .songs
                    .iter()
                    .map(|song| {
                        let mut item = song.number.to_string();
                        item.push_str(" ");
                        item.push_str(&song.title);
                        ListItem::new(item)
                    })
                    .collect();

                self.list_size = list.len() - 1;
                self.list = list;
            }
        }
    }
    pub fn exit_mode(&mut self) {
        match self.mode {
            Mode::Artist => return,
            Mode::Album => self.mode = Mode::Artist,
            Mode::Track => self.mode = Mode::Album,
        }
        self.update_mode();
        self.selected = 0;
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
                    }
                    | KeyEvent {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::NONE,
                    } => {
                        if self.selected != self.list_size {
                            self.selected += 1;
                        } else {
                            self.selected = 0;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::NONE,
                    } => {
                        if self.selected != 0 {
                            self.selected -= 1;
                        } else {
                            self.selected = self.list_size;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::NONE,
                    } => self.change_mode(),
                    KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('h'),
                        modifiers: KeyModifiers::NONE,
                    } => self.exit_mode(),
                    _ => (),
                },
                Event::Mouse(_) => (),
                Event::Resize(_, _) => (),
            }
        }
        Ok(())
    }
}

impl<'a> Drop for App<'a> {
    fn drop(&mut self) {
        execute!(stdout(), LeaveAlternateScreen).unwrap();
    }
}

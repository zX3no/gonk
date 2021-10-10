use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::io::stdout;
use std::ops::Index;
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

pub struct App {
    music: HashMap<String, Artist>,
    //this needs to change
    mode: Mode,
    artist_list: Vec<String>,
    album_list: Vec<String>,
    track_list: Vec<String>,
    length: usize,

    selected_artist: String,
    selected_album: String,
    state: ListState,
    //this is very dumb
    selected: usize,
    quit: bool,
}
impl App {
    pub fn new() -> Self {
        execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();
        let music = get_artists();

        Self {
            music,
            mode: Mode::Artist,
            artist_list: Vec::new(),
            album_list: Vec::new(),
            track_list: Vec::new(),
            length: 0,
            selected_artist: String::new(),
            selected_album: String::new(),
            selected: 0,
            state: ListState::default(),
            quit: false,
        }
    }
    pub fn run(&mut self) -> Result {
        let mut terminal =
            Terminal::new(CrosstermBackend::new(stdout())).expect("couldn't create terminal");

        terminal.clear().unwrap();

        self.update_lists();
        loop {
            terminal
                .draw(|f| {
                    let size = f.size();

                    let list = match self.mode {
                        Mode::Artist => &self.artist_list,
                        Mode::Album => &self.album_list,
                        Mode::Track => &self.track_list,
                    };

                    let list: Vec<ListItem> = list
                        .iter()
                        .map(|item| ListItem::new(item.clone()))
                        .collect();

                    let l = List::new(list)
                        .block(Block::default().title("Artists").borders(Borders::ALL))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                        .highlight_symbol(">>");

                    let left = Rect::new(0, 0, size.width / 3, size.height);

                    let b = Block::default().title("Block").borders(Borders::ALL);
                    let right =
                        Rect::new(size.width / 3, 0, size.width - size.width / 3, size.height);

                    self.state.select(Some(self.selected));
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
    pub fn play(&mut self) {
        // let path = &self
        //     .music
        //     .get(&self.artist)
        //     .unwrap()
        //     .album(&self.album)
        //     .unwrap()
        //     .track(self.selected as u16 + 1)
        //     .unwrap()
        //     .path;

        // let p = path.clone();

        // thread::spawn(move || {
        //     Player::play(&p);
        // });
    }
    pub fn update_lists(&mut self) {
        match self.mode {
            Mode::Artist => {
                self.artist_list = self.music.iter().map(|(_, v)| v.name.clone()).collect();
                self.length = self.artist_list.len() - 1;

                if !self.selected_artist.is_empty() {
                    let index = self
                        .artist_list
                        .iter()
                        .position(|a| a == &self.selected_artist)
                        .unwrap();

                    self.selected = index;
                }
                self.selected_artist = String::new();
                self.selected_album = String::new();
            }
            Mode::Album => {
                if self.selected_artist.is_empty() && self.selected_album.is_empty() {
                    //we just came from the artist page and haven't selected an album yet
                    //remember the artist we're selecting
                    self.selected_artist = self.artist_list.get(self.selected).unwrap().clone();
                    self.album_list = self
                        .music
                        .get(&self.selected_artist)
                        .unwrap()
                        .albums
                        .iter()
                        .map(|album| album.title.clone())
                        .collect();

                    self.length = self.album_list.len() - 1;
                    self.selected = 0;
                } else if !self.selected_album.is_empty() {
                    //we are returning from track mode
                    //so just update the index
                    let index = self
                        .album_list
                        .iter()
                        .position(|a| a == &self.selected_album)
                        .unwrap();

                    self.selected = index;

                    //remeber to remove the album we just selected
                    self.selected_album = String::new();
                } else {
                    panic!();
                }
            }
            Mode::Track => {
                if self.selected_album.is_empty() {
                    self.selected_album = self.album_list.get(self.selected).unwrap().clone();
                }
                self.track_list = self
                    .music
                    .get(&self.selected_artist)
                    .unwrap()
                    .album(&self.selected_album)
                    .unwrap()
                    .songs
                    .iter()
                    .map(|song| song.title.clone())
                    .collect();

                self.selected_album = self.album_list.get(self.selected).unwrap().clone();
                self.length = self.track_list.len() - 1;
                self.selected = 0;
            }
        }
    }
    pub fn exit_mode(&mut self) {
        match self.mode {
            Mode::Artist => return,
            Mode::Album => self.mode = Mode::Artist,
            Mode::Track => self.mode = Mode::Album,
        }
        self.update_lists();
    }

    pub fn select_item(&mut self) {
        //user pressed enter on item
        match self.mode {
            Mode::Artist => self.mode = Mode::Album,
            Mode::Album => self.mode = Mode::Track,
            //todo move track onto player
            Mode::Track => return,
        }
        self.update_lists();
    }
    pub fn search(&mut self) {}
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
                        if self.selected != self.length {
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
                            self.selected = self.length;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::NONE,
                    } => self.select_item(),
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

impl Drop for App {
    fn drop(&mut self) {
        execute!(stdout(), LeaveAlternateScreen).unwrap();
    }
}

use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use std::collections::HashMap;
use std::io::stdout;
use std::ops::Index;
use std::thread;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, BorderType, Borders, ListItem, ListState};
use tui::Terminal;

use crossterm::execute;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use crate::database::Artist;
use crate::index::get_artists;
use crate::list::List;
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
    artist_list: List,
    album_list: List,
    track_list: List,
    state: ListState,
    quit: bool,
}
impl App {
    pub fn new() -> Self {
        // execute!(stdout(), EnterAlternateScreen).unwrap();
        enable_raw_mode().unwrap();
        let music = get_artists();

        let artist_list = List::from_vec(music.iter().map(|(_, v)| v.name.clone()).collect());

        Self {
            music,
            mode: Mode::Artist,
            artist_list,
            album_list: List::new(),
            track_list: List::new(),
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

                    let list = match self.mode {
                        Mode::Artist => {
                            self.state.select(Some(self.artist_list.selection));
                            &self.artist_list.items
                        }
                        Mode::Album => {
                            self.state.select(Some(self.album_list.selection));
                            &self.album_list.items
                        }
                        Mode::Track => {
                            self.state.select(Some(self.track_list.selection));
                            &self.track_list.items
                        }
                    };

                    let list: Vec<ListItem> = list
                        .iter()
                        .map(|item| ListItem::new(item.clone()))
                        .collect();

                    //todo update the title for each page
                    let l = tui::widgets::List::new(list)
                        .block(Block::default().title("Artists").borders(Borders::ALL))
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
    pub fn exit_mode(&mut self) {
        match self.mode {
            Mode::Artist => {}
            Mode::Album => {
                //exit to artist mode
                self.mode = Mode::Artist;

                //we want to be on album 0 next time we change modes
                self.album_list.clear_selection();
            }
            Mode::Track => {
                self.mode = Mode::Album;

                //we want to be on track 0 next time we change modes
                self.track_list.clear_selection();
            }
        }
    }
    pub fn get_albums(&self, artist: &String) -> Vec<String> {
        self.music
            .get(artist)
            .unwrap()
            .albums
            .iter()
            .map(|album| album.title.clone())
            .collect()
    }
    pub fn get_tracks(&self, artist: &String, album: &String) -> Vec<String> {
        self.music
            .get(artist)
            .unwrap()
            .album(&album)
            .unwrap()
            .songs
            .iter()
            .map(|song| song.title.clone())
            .collect()
    }

    pub fn select_item(&mut self) {
        //user pressed enter on item
        match self.mode {
            //into album
            Mode::Artist => {
                //update renderer
                self.mode = Mode::Album;

                //update the albums
                let artist = self.artist_list.selected();
                self.album_list = List::from_vec(self.get_albums(&artist));
            }
            //track
            Mode::Album => {
                self.mode = Mode::Track;

                //update the tracks
                let artist = self.artist_list.selected();
                let album = self.album_list.selected();
                self.track_list = List::from_vec(self.get_tracks(&artist, &album));
            }
            //play track
            Mode::Track => return,
        }
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
                    } => match self.mode {
                        Mode::Artist => self.artist_list.down(),
                        Mode::Album => self.album_list.down(),
                        Mode::Track => self.track_list.down(),
                    },
                    KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::NONE,
                    } => match self.mode {
                        Mode::Artist => self.artist_list.up(),
                        Mode::Album => self.album_list.up(),
                        Mode::Track => self.track_list.up(),
                    },
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

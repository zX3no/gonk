use crate::widget::{List, ListItem, ListState};
use gonk_server::Client;
use std::{cell::RefCell, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders},
    Frame,
};

#[derive(Debug)]
pub enum Mode {
    Artist,
    Album,
    Song,
}

impl Mode {
    pub fn next(&mut self) {
        match self {
            Mode::Artist => *self = Mode::Album,
            Mode::Album => *self = Mode::Song,
            Mode::Song => (),
        }
    }
    pub fn prev(&mut self) {
        match self {
            Mode::Artist => (),
            Mode::Album => *self = Mode::Artist,
            Mode::Song => *self = Mode::Album,
        }
    }
}

pub struct Browser {
    pub mode: Mode,
    client: Rc<RefCell<Client>>,
}

impl Browser {
    pub fn new(client: Rc<RefCell<Client>>) -> Self {
        optick::event!("new browser");

        Self {
            mode: Mode::Artist,
            client,
        }
    }
    pub fn on_enter(&self) {
        let mut client = self.client.borrow_mut();

        //TODO: we can send the ids but we should also copy the data over to the queue
        if let Some(artist) = client.artists.selected().cloned() {
            if let Some(song) = client.songs.selected() {
                match self.mode {
                    Mode::Artist => {
                        client.play_artist(artist);
                    }
                    Mode::Album => {
                        let ids: Vec<u64> = client
                            .songs
                            .data
                            .iter()
                            .filter_map(|song| song.id)
                            .collect();
                        client.add_ids(&ids);
                    }
                    Mode::Song => {
                        if let Some(id) = song.id {
                            client.add_ids(&[id]);
                        }
                    }
                }
            }
        }
    }
    pub fn prev(&mut self) {
        self.mode.prev();
    }
    pub fn next(&mut self) {
        self.mode.next();
    }
    pub fn up(&mut self) {
        let mut client = self.client.borrow_mut();
        match self.mode {
            Mode::Artist => {
                client.artists.up();
            }
            Mode::Album => {
                client.albums.up();
            }
            Mode::Song => {
                client.songs.up();
            }
        }
        drop(client);
        self.update();
    }
    pub fn down(&mut self) {
        let mut client = self.client.borrow_mut();
        match self.mode {
            Mode::Artist => {
                client.artists.down();
            }
            Mode::Album => {
                client.albums.down();
            }
            Mode::Song => {
                client.songs.down();
            }
        }
        drop(client);
        self.update();
    }
    pub fn update(&mut self) {
        let mut client = self.client.borrow_mut();
        match self.mode {
            Mode::Artist => {
                if let Some(artist) = client.artists.selected().cloned() {
                    client.update_artist(artist);
                }
            }
            Mode::Album => {
                if let Some(artist) = client.artists.selected().cloned() {
                    if let Some(album) = client.albums.selected().cloned() {
                        client.update_album(album, artist);
                    }
                }
            }
            Mode::Song => (),
        }
    }
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        optick::event!("draw Browser");

        let area = f.size();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ]
                .as_ref(),
            )
            .split(area);

        let client = self.client.borrow();

        let a: Vec<_> = client
            .artists
            .data
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let b: Vec<_> = client
            .albums
            .data
            .iter()
            .map(|name| ListItem::new(name.as_str()))
            .collect();

        let c: Vec<_> = client
            .songs
            .data
            .iter()
            .map(|song| ListItem::new(format!("{}. {}", song.number, song.name)))
            .collect();

        let artists = List::new(a)
            .block(
                Block::default()
                    .title("─Aritst")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut artist_state = ListState::new(client.artists.index);

        let albums = List::new(b)
            .block(
                Block::default()
                    .title("─Album")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut album_state = ListState::new(client.albums.index);

        let songs = List::new(c)
            .block(
                Block::default()
                    .title("─Song")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default())
            .highlight_symbol(">");

        let mut song_state = ListState::new(client.songs.index);

        //TODO: better way of doing this?
        match self.mode {
            Mode::Artist => {
                album_state.select(None);
                song_state.select(None);
            }
            Mode::Album => {
                artist_state.select(None);
                song_state.select(None);
            }
            Mode::Song => {
                artist_state.select(None);
                album_state.select(None);
            }
        }

        f.render_stateful_widget(artists, chunks[0], &mut artist_state);
        f.render_stateful_widget(albums, chunks[1], &mut album_state);
        f.render_stateful_widget(songs, chunks[2], &mut song_state);
    }
}

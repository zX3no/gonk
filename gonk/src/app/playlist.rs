#![allow(unused)]
use crate::widgets::{Cell, Gauge, List, ListItem, ListState, Row, Table, TableState};
use gonk_core::Index;
use std::io::Stdout;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, BorderType, Borders},
    Frame,
};

enum Mode {
    Playlist,
    Song,
}

pub struct Playlist<'a> {
    mode: Mode,
    playlist: Index<ListItem<'a>>,
    songs: Index<Row<'a>>,
}

impl<'a> Playlist<'a> {
    pub fn new() -> Self {
        Self {
            mode: Mode::Playlist,
            playlist: Index::new(
                vec![
                    ListItem::new("Playlist 1"),
                    ListItem::new("Playlist 2"),
                    ListItem::new("Playlist 3"),
                    ListItem::new("Playlist 4"),
                    ListItem::new("Playlist 5"),
                ],
                Some(0),
            ),
            songs: Index::new(vec![Row::new(["1", "Title", "Album", "Artist"])], Some(0)),
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            Mode::Playlist => self.playlist.up(),
            Mode::Song => self.songs.up(),
        }
    }
    pub fn down(&mut self) {
        match self.mode {
            Mode::Playlist => self.playlist.down(),
            Mode::Song => self.songs.down(),
        }
    }
    pub fn on_enter(&mut self) {
        self.right();
    }
    pub fn on_backspace(&mut self) {
        self.left();
    }
    pub fn left(&mut self) {
        if let Mode::Song = self.mode {
            self.mode = Mode::Playlist;
        }
    }
    pub fn right(&mut self) {
        if let Mode::Playlist = self.mode {
            self.mode = Mode::Song;
            self.songs.select(Some(0));
        }
    }
}

//A list of every playlist on the left
//Then the content on the right
//| ... |              |
//|     |              |
//| ... |              |
//|     |              |
//| ... |              |
//|____________________|
//|____________________|
//The should be a bar at the bottom with a list of controls
//Rename, Delete, Remove from playlist, move song up, move song down

impl<'a> Playlist<'a> {
    pub fn draw(&self, f: &mut Frame<CrosstermBackend<Stdout>>) {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)])
            .split(f.size());

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(vertical[0]);

        let list = List::new(self.playlist.clone())
            .block(
                Block::default()
                    .title("─Playlist")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .highlight_symbol(">");

        let list = if let Mode::Playlist = self.mode {
            list.highlight_symbol(">")
        } else {
            list.highlight_symbol("")
        };

        f.render_stateful_widget(
            list,
            horizontal[0],
            &mut ListState::new(self.playlist.index()),
        );

        let table = Table::new(self.songs.clone())
            .header(Row::new(["Track", "Title", "Album", "Artist"]).bottom_margin(1))
            .widths(&[
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .block(
                Block::default()
                    .title("─Songs")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );

        let table = if let Mode::Song = self.mode {
            table.highlight_symbol(">")
        } else {
            table.highlight_symbol("")
        };

        f.render_stateful_widget(
            table,
            horizontal[1],
            &mut TableState::new(self.songs.index()),
        );

        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            vertical[1],
        );
    }
}

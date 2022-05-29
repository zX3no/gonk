#![allow(unused)]
use crate::widgets::{Cell, Gauge, List, ListItem, ListState, Row, Table, TableState};
use gonk_core::{sqlite, Index, Song};
use gonk_player::Player;
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

pub struct Item {
    id: usize,
    row: usize,
    song: Song,
}

pub struct Playlist {
    mode: Mode,
    playlist: Index<String>,
    songs: Index<Item>,
}

impl Playlist {
    pub fn new() -> Self {
        let playlists = sqlite::playlist::get_names();
        let songs = Playlist::get_songs(playlists.first());

        Self {
            mode: Mode::Playlist,
            playlist: Index::new(playlists, Some(0)),
            songs: Index::new(songs, Some(0)),
        }
    }
    fn get_songs(playlist: Option<&String>) -> Vec<Item> {
        if let Some(playlist) = playlist {
            let (row_ids, song_ids) = sqlite::playlist::get(playlist);
            let songs = sqlite::get_songs_from_id(&song_ids);
            songs
                .into_iter()
                .zip(song_ids)
                .zip(row_ids)
                .map(|((song, id), row)| Item { id, row, song })
                .collect()
        } else {
            Vec::new()
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            Mode::Playlist => {
                self.playlist.up();
                self.update_songs();
            }
            Mode::Song => self.songs.up(),
        }
    }
    pub fn down(&mut self) {
        match self.mode {
            Mode::Playlist => {
                self.playlist.down();
                self.update_songs();
            }
            Mode::Song => self.songs.down(),
        }
    }
    pub fn update_songs(&mut self) {
        //Update the list of songs.
        let songs = Playlist::get_songs(self.playlist.selected());
        self.songs = if !songs.is_empty() {
            Index::new(songs, Some(0))
        } else {
            self.mode = Mode::Playlist;
            Index::default()
        };
    }
    pub fn on_enter(&mut self, player: &mut Player) {
        match self.mode {
            Mode::Playlist => self.right(),
            Mode::Song => {
                if let Some(item) = self.songs.selected() {
                    player.add_songs(&[item.song.clone()]);
                }
            }
        }
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
            if !self.songs.is_empty() {
                self.mode = Mode::Song;
            }
        }
    }
    pub fn add_to_playlist(&mut self, name: &str, songs: &[usize]) {
        sqlite::add_playlist(name, songs);

        //TODO: I really hate how wasteful these refreshes are.
        self.playlist = Index::new(sqlite::playlist::get_names(), self.playlist.index());
        self.update_songs();
    }
    pub fn delete(&mut self) {
        match self.mode {
            Mode::Playlist => (),
            Mode::Song => {
                if let Some(song) = self.songs.selected() {
                    sqlite::playlist::remove(song.row);
                    self.update_songs();
                }
            }
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

impl Playlist {
    pub fn draw(&self, f: &mut Frame<CrosstermBackend<Stdout>>) {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(3)])
            .split(f.size());

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(vertical[0]);

        let items: Vec<ListItem> = self
            .playlist
            .data
            .clone()
            .into_iter()
            .map(|str| ListItem::new(str))
            .collect();

        let list = List::new(items)
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

        let content = self
            .songs
            .data
            .iter()
            .map(|item| {
                let song = item.song.clone();
                Row::new(vec![
                    song.number.to_string(),
                    song.name,
                    song.album,
                    song.artist,
                ])
            })
            .collect();

        let table = Table::new(content)
            // .header(Row::new(["Track", "Title", "Album", "Artist"]).bottom_margin(1))
            .widths(&[
                Constraint::Length(8),
                Constraint::Length(42),
                Constraint::Length(24),
                Constraint::Length(26),
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

#![allow(unused)]
use crate::widgets::{Cell, List, ListItem, ListState, Row, Table, TableState};
use crossterm::event::KeyModifiers;
use gonk::{centered_rect, Frame};
use gonk_core::{sqlite, Index, Song};
use gonk_player::Player;
use std::io::Stdout;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

#[derive(PartialEq, Eq)]
enum Mode {
    Playlist,
    Song,
    Popup,
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
    songs_to_add: Vec<Song>,
    search: String,
    changed: bool,
    search_results: Index<String>,
}

impl Playlist {
    pub fn new() -> Self {
        let playlists = sqlite::playlist::get_names();
        let songs = Playlist::get_songs(playlists.first());

        Self {
            mode: Mode::Playlist,
            playlist: Index::new(playlists, Some(0)),
            songs: Index::new(songs, Some(0)),
            songs_to_add: Vec::new(),
            changed: false,
            search: String::new(),
            search_results: Index::default(),
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
            Mode::Popup => (),
        }
    }
    pub fn down(&mut self) {
        match self.mode {
            Mode::Playlist => {
                self.playlist.down();
                self.update_songs();
            }
            Mode::Song => self.songs.down(),
            Mode::Popup => (),
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
            Mode::Playlist => {
                let songs: Vec<Song> = self
                    .songs
                    .data
                    .iter()
                    .map(|item| item.song.clone())
                    .collect();

                player.add_songs(&songs);
            }
            Mode::Song => {
                if let Some(item) = self.songs.selected() {
                    player.add_songs(&[item.song.clone()]);
                }
            }
            Mode::Popup => {
                let name = if let Some(name) = self.search_results.data.first() {
                    name
                } else {
                    &self.search
                };
                //TODO: Get id from Song
                sqlite::add_playlist(name, &[10]);

                //TODO: I really hate how wasteful these refreshes are.
                self.playlist = Index::new(sqlite::playlist::get_names(), self.playlist.index());
                self.update_songs();

                //TODO: Select the current playlist
                self.mode = Mode::Song;
            }
        }
    }
    pub fn on_backspace(&mut self, modifiers: KeyModifiers) {
        match self.mode {
            Mode::Popup => {
                if modifiers == KeyModifiers::CONTROL {
                    self.search.clear();
                } else {
                    self.search.pop();
                }
            }
            _ => self.left(),
        }
    }
    pub fn left(&mut self) {
        match self.mode {
            Mode::Song => {
                self.mode = Mode::Playlist;
            }
            Mode::Popup => (),
            _ => (),
        }
    }
    pub fn right(&mut self) {
        match self.mode {
            Mode::Playlist if !self.songs.is_empty() => {
                self.mode = Mode::Song;
            }
            Mode::Popup => (),
            _ => (),
        }
    }
    pub fn add_to_playlist(&mut self, songs: &[Song]) {
        self.songs_to_add = songs.to_vec();
        self.mode = Mode::Popup;
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
            Mode::Popup => (),
        }
    }
    pub fn on_key(&mut self, c: char) {
        self.changed = true;
        self.search.push(c);
    }
    pub fn input_mode(&self) -> bool {
        self.mode == Mode::Popup
    }
}

impl Playlist {
    pub fn draw_popup(&mut self, f: &mut Frame) {
        //TODO: Draw a search box that searches
        //all of the avaliable playlist names
        //if none is found it will make a new one

        if let Some(area) = centered_rect(45, 23, f.size()) {
            let v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Percentage(50)])
                .margin(1)
                .split(area);

            f.render_widget(Clear, area);
            f.render_widget(
                Block::default()
                    .title("─Select a playlist")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
                area,
            );

            let text = if self.search.is_empty() {
                "Enter playlist name.."
            } else {
                self.search.as_str()
            };
            f.render_widget(
                Paragraph::new(text).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                ),
                v[0],
            );

            if self.changed {
                self.changed = false;
                self.search_results.data = self
                    .playlist
                    .data
                    .iter()
                    .filter(|str| self.search.is_empty() || str.contains(&self.search))
                    .map(|str| str.to_string())
                    .collect();
            }

            let mut items: Vec<ListItem> = self
                .search_results
                .data
                .iter()
                .map(|str| ListItem::new(str.as_str()))
                .collect();

            if items.is_empty() && !self.search.is_empty() {
                items.push(ListItem::new(format!("Add new {}", self.search)))
            }

            let list = List::new(items);
            f.render_widget(
                list,
                v[1].inner(&Margin {
                    horizontal: 1,
                    vertical: 0,
                }),
            );
        }
    }
    pub fn draw(&mut self, f: &mut Frame) {
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

        self.draw_footer(f, vertical[1]);

        if let Mode::Popup = self.mode {
            self.draw_popup(f);
        }
    }
    pub fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let keys = "[Enter] Add [X] Delete [CTRL + R] Rename [SHIFT + K] Up [SHIFT + J] Down";

        f.render_widget(
            Paragraph::new(keys).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            area,
        );
    }
}

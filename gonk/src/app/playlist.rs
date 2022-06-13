#![allow(unused)]
use crate::widgets::{centered_rect, Cell, List, ListItem, ListState, Row, Table, TableState};
use crate::{sqlite, Frame};
use crossterm::event::KeyModifiers;
use gonk_player::{Index, Player, Song};
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
    search_result: String,
    changed: bool,
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
            search_result: String::from("Enter a playlist name..."),
        }
    }
    fn get_songs(playlist: Option<&String>) -> Vec<Item> {
        if let Some(playlist) = playlist {
            let (row_ids, song_ids) = sqlite::playlist::get(playlist);
            let songs = sqlite::get_songs(&song_ids);
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
    fn update_songs(&mut self) {
        //Update the list of songs.
        let songs = Playlist::get_songs(self.playlist.selected());
        self.songs = if !songs.is_empty() {
            Index::new(songs, self.songs.index())
        } else {
            self.mode = Mode::Playlist;
            Index::default()
        };
    }
    fn update_playlists(&mut self) {
        self.playlist = Index::new(sqlite::playlist::get_names(), self.playlist.index());
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
            Mode::Popup if !self.songs_to_add.is_empty() => {
                //Select an existing playlist or create a new one.
                let name = self.search.trim().to_string();

                let ids: Vec<usize> = self
                    .songs_to_add
                    .iter()
                    .map(|song| song.id.unwrap())
                    .collect();

                sqlite::add_playlist(&name, &ids);

                self.update_playlists();

                let mut i = Some(0);
                for (j, playlist) in self.playlist.data.iter().enumerate() {
                    if playlist == &name {
                        i = Some(j);
                        break;
                    }
                }
                //Select the playlist was just modified and update the songs.
                self.playlist.select(i);
                self.update_songs();

                //Reset everything.
                self.search = String::new();
                self.mode = Mode::Song;
            }
            _ => (),
        }
    }
    pub fn on_backspace(&mut self, modifiers: KeyModifiers) {
        match self.mode {
            Mode::Popup => {
                self.changed = true;
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
            Mode::Playlist => {
                if let Some(playlist) = self.playlist.selected() {
                    //TODO: Prompt the user with yes or no.
                    sqlite::playlist::remove(&playlist);

                    let index = self.playlist.index().unwrap();
                    self.playlist.remove(index);
                    self.update_songs();
                }
            }
            Mode::Song => {
                if let Some(song) = self.songs.selected() {
                    sqlite::playlist::remove_id(song.row);
                    let index = self.songs.index().unwrap();
                    self.songs.remove(index);
                    if self.songs.is_empty() {
                        self.update_playlists();
                    }
                }
            }
            Mode::Popup => return,
        }
    }
    pub fn on_key(&mut self, c: char) {
        self.changed = true;
        self.search.push(c);
    }
    pub fn input_mode(&self) -> bool {
        self.mode == Mode::Popup
    }
    pub fn on_escape(&mut self, mode: &mut super::Mode) {
        match self.mode {
            Mode::Popup => {
                self.mode = Mode::Playlist;
                self.search = String::new();
                self.changed = true;
            }
            _ => *mode = super::Mode::Browser,
        };
    }
}

impl Playlist {
    //TODO: I think I want a different popup.
    //It should be a small side bar in the browser.
    //There should be a list of existing playlists.
    //The first playlist will be the one you just added to
    //so it's fast to keep adding things
    //The last item will be add a new playlist.
    //If there are no playlists it will prompt you to create on.
    //This should be similar to foobar on android.

    //TODO: Renaming
    //Move items around in lists
    //There should be a hotkey to add to most recent playlist
    //And a message should show up in the bottom bar saying
    //"[name] has been has been added to [playlist name]"
    //or
    //"25 songs have been added to [playlist name]"

    //TODO: Add keybindings to readme
    pub fn draw_popup(&mut self, f: &mut Frame) {
        if let Some(area) = centered_rect(45, 6, f.size()) {
            let v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Percentage(50)])
                .margin(1)
                .split(area);

            f.render_widget(Clear, area);
            f.render_widget(
                Block::default()
                    .title("─Add to playlist")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
                area,
            );

            //Scroll the playlist name.
            let len = self.search.len() as u16;
            let width = v[0].width.saturating_sub(1);
            let offset_x = if len < width { 0 } else { len - width + 1 };

            f.render_widget(
                Paragraph::new(self.search.as_str())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    )
                    .scroll((0, offset_x)),
                v[0],
            );

            let mut items: Vec<ListItem> = self
                .playlist
                .data
                .iter()
                .map(|str| ListItem::new(str.as_str()))
                .collect();

            if self.changed {
                self.changed = false;
                let eq = self.playlist.data.iter().any(|e| e == &self.search);
                self.search_result = if eq {
                    format!("Add to existing playlist: {}", self.search)
                } else if self.search.is_empty() {
                    String::from("Enter a playlist name...")
                } else {
                    format!("Add to new playlist: {}", self.search)
                }
            }

            f.render_widget(
                Paragraph::new(self.search_result.as_str()),
                v[1].inner(&Margin {
                    horizontal: 1,
                    vertical: 0,
                }),
            );

            //Draw the cursor.
            let (x, y) = (v[0].x + 1, v[0].y + 1);
            if self.search.is_empty() {
                f.set_cursor(x, y);
            } else {
                let width = v[0].width.saturating_sub(3);
                if len < width {
                    f.set_cursor(x + len, y)
                } else {
                    f.set_cursor(x + width, y)
                }
            }
        }
    }
    pub fn draw(&mut self, area: Rect, f: &mut Frame) {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

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
                Row::new(vec![song.name, song.album, song.artist])
            })
            .collect();

        let table = Table::new(content)
            .widths(&[
                Constraint::Percentage(42),
                Constraint::Percentage(30),
                Constraint::Percentage(28),
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

        if let Mode::Popup = self.mode {
            self.draw_popup(f);
        }
    }
}

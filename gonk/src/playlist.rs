use crate::{widgets::*, Frame, Input, COLORS};
use gonk_database::{playlist, query};
use gonk_player::{Index, Player, Song};
use tui::style::Style;
use tui::text::Span;
use tui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

#[derive(PartialEq, Eq)]
pub enum Mode {
    Playlist,
    Song,
    Popup,
}

pub struct Item {
    row: usize,
    song: Song,
}

//FIXME: Playlist songs are currently stored by id.
//If the database is rescanned it will delete and re-add every song.
//This can change the rowid's so the songs can be replaced with something else.
//Playlist songs should be stored the same way normal songs.

pub struct Playlist {
    pub mode: Mode,
    pub titles: Index<String>,
    pub songs: Index<Item>,
    pub songs_to_add: Vec<Song>,
    pub search: String,
    pub search_result: String,
    pub changed: bool,
}

impl Playlist {
    pub fn new() -> Self {
        let playlists = playlist::get_names();
        let songs = songs(playlists.first());

        Self {
            mode: Mode::Playlist,
            titles: Index::new(playlists, Some(0)),
            songs: Index::new(songs, Some(0)),
            songs_to_add: Vec::new(),
            changed: false,
            search: String::new(),
            search_result: String::from("Enter a playlist name..."),
        }
    }
}

impl Input for Playlist {
    fn up(&mut self) {
        match self.mode {
            Mode::Playlist => {
                self.titles.up();
                update_songs(self);
            }
            Mode::Song => self.songs.up(),
            Mode::Popup => (),
        }
    }

    fn down(&mut self) {
        match self.mode {
            Mode::Playlist => {
                self.titles.down();
                update_songs(self);
            }
            Mode::Song => self.songs.down(),
            Mode::Popup => (),
        }
    }

    fn left(&mut self) {
        if self.mode == Mode::Song {
            self.mode = Mode::Playlist;
        }
    }

    fn right(&mut self) {
        match self.mode {
            Mode::Playlist if !self.songs.is_empty() => {
                self.mode = Mode::Song;
            }
            _ => (),
        }
    }
}

fn songs(playlist: Option<&String>) -> Vec<Item> {
    if let Some(playlist) = playlist {
        let (row_ids, song_ids) = playlist::get(playlist);
        let songs = query::songs_from_ids(&song_ids);
        songs
            .into_iter()
            .zip(row_ids)
            .map(|(song, row)| Item { row, song })
            .collect()
    } else {
        Vec::new()
    }
}

fn update_songs(playlist: &mut Playlist) {
    //Update the list of songs.
    let songs = songs(playlist.titles.selected());
    playlist.songs = if songs.is_empty() {
        playlist.mode = Mode::Playlist;
        Index::default()
    } else {
        Index::new(songs, playlist.songs.index())
    };
}

pub fn on_enter(playlist: &mut Playlist, player: &mut Player) {
    match playlist.mode {
        Mode::Playlist => {
            let songs: Vec<Song> = playlist
                .songs
                .data
                .iter()
                .map(|item| &item.song)
                .cloned()
                .collect();

            player.add_songs(&songs);
        }
        Mode::Song => {
            if let Some(item) = playlist.songs.selected() {
                player.add_songs(&[item.song.clone()]);
            }
        }
        Mode::Popup if !playlist.songs_to_add.is_empty() => {
            //Select an existing playlist or create a new one.
            let name = playlist.search.trim().to_string();

            let ids: Vec<usize> = playlist
                .songs_to_add
                .iter()
                .map(|song| song.id.unwrap())
                .collect();

            playlist::add(&name, &ids);

            playlist.titles = Index::new(playlist::get_names(), playlist.titles.index());

            let mut i = Some(0);
            for (j, playlist) in playlist.titles.data.iter().enumerate() {
                if playlist == &name {
                    i = Some(j);
                    break;
                }
            }
            //Select the playlist was just modified and update the songs.
            playlist.titles.select(i);
            update_songs(playlist);

            //Reset everything.
            playlist.search = String::new();
            playlist.mode = Mode::Song;
        }
        Mode::Popup => (),
    }
}

pub fn on_backspace(playlist: &mut Playlist, control: bool) {
    match playlist.mode {
        Mode::Popup => {
            playlist.changed = true;
            if control {
                playlist.search.clear();
            } else {
                playlist.search.pop();
            }
        }
        _ => playlist.left(),
    }
}

pub fn add_to_playlist(playlist: &mut Playlist, songs: &[Song]) {
    playlist.songs_to_add = songs.to_vec();
    playlist.mode = Mode::Popup;
}

pub fn delete(playlist: &mut Playlist) {
    match playlist.mode {
        Mode::Playlist => {
            if let Some(selected) = playlist.titles.selected() {
                //TODO: Prompt the user with yes or no.
                playlist::remove(selected);

                let index = playlist.titles.index().unwrap();
                playlist.titles.remove(index);
                update_songs(playlist);
            }
        }
        Mode::Song => {
            if let Some(song) = playlist.songs.selected() {
                playlist::remove_id(song.row);
                let index = playlist.songs.index().unwrap();
                playlist.songs.remove(index);
                if playlist.songs.is_empty() {
                    playlist.titles = Index::new(playlist::get_names(), playlist.titles.index());
                }
            }
        }
        Mode::Popup => (),
    }
}

pub fn on_escape(playlist: &mut Playlist, mode: &mut super::Mode) {
    match playlist.mode {
        Mode::Popup => {
            playlist.mode = Mode::Playlist;
            playlist.search = String::new();
            playlist.changed = true;
        }
        _ => *mode = super::Mode::Browser,
    };
}

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
pub fn draw_popup(playlist: &mut Playlist, f: &mut Frame) {
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
        let len = playlist.search.len() as u16;
        let width = v[0].width.saturating_sub(1);
        let offset_x = if len < width { 0 } else { len - width + 1 };

        f.render_widget(
            Paragraph::new(playlist.search.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .scroll((0, offset_x)),
            v[0],
        );

        if playlist.changed {
            playlist.changed = false;
            let eq = playlist.titles.data.iter().any(|e| e == &playlist.search);
            playlist.search_result = if eq {
                format!("Add to existing playlist: {}", playlist.search)
            } else if playlist.search.is_empty() {
                String::from("Enter a playlist name...")
            } else {
                format!("Add to new playlist: {}", playlist.search)
            }
        }

        f.render_widget(
            Paragraph::new(playlist.search_result.as_str()),
            v[1].inner(&Margin {
                horizontal: 1,
                vertical: 0,
            }),
        );

        //Draw the cursor.
        let (x, y) = (v[0].x + 1, v[0].y + 1);
        if playlist.search.is_empty() {
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

pub fn draw(playlist: &mut Playlist, area: Rect, f: &mut Frame) {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<ListItem> = playlist
        .titles
        .clone()
        .into_iter()
        .map(ListItem::new)
        .collect();

    let list = List::new(&items)
        .block(
            Block::default()
                .title("─Playlist")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_symbol(">");

    let list = if let Mode::Playlist = playlist.mode {
        list.highlight_symbol(">")
    } else {
        list.highlight_symbol("")
    };

    f.render_stateful_widget(
        list,
        horizontal[0],
        &mut ListState::new(playlist.titles.index()),
    );

    let content = playlist
        .songs
        .data
        .iter()
        .map(|item| {
            let song = item.song.clone();
            Row::new(vec![
                Span::styled(song.name, Style::default().fg(COLORS.name)),
                Span::styled(song.album, Style::default().fg(COLORS.album)),
                Span::styled(song.artist, Style::default().fg(COLORS.artist)),
            ])
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

    let table = if let Mode::Song = playlist.mode {
        table.highlight_symbol(">")
    } else {
        table.highlight_symbol("")
    };

    f.render_stateful_widget(
        table,
        horizontal[1],
        &mut TableState::new(playlist.songs.index()),
    );

    if let Mode::Popup = playlist.mode {
        draw_popup(playlist, f);
    }
}

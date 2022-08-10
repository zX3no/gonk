use crate::{log, save_queue, widgets::*, Frame, Input, COLORS};
use crossterm::event::MouseEvent;
use gonk_database::{Index, RawPlaylist, RawSong, Song};
use gonk_player::Player;
use tui::layout::Alignment;
use tui::style::{Color, Modifier, Style};
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

pub struct Playlist {
    pub mode: Mode,
    pub playlists: Index<RawPlaylist>,
    pub song_buffer: Vec<Song>,
    pub search_query: String,
    pub search_result: String,
    pub changed: bool,

    pub delete: bool,
    pub yes: bool,
}

impl Playlist {
    pub fn new() -> Self {
        let playlists = gonk_database::playlists();

        Self {
            mode: Mode::Playlist,
            playlists: Index::from(playlists),
            song_buffer: Vec::new(),
            changed: false,
            search_query: String::new(),
            search_result: String::from("Enter a playlist name..."),

            delete: false,
            yes: true,
        }
    }
}

impl Input for Playlist {
    fn up(&mut self) {
        if !self.delete {
            match self.mode {
                Mode::Playlist => {
                    self.playlists.up();
                }
                Mode::Song => {
                    if let Some(selected) = self.playlists.selected_mut() {
                        selected.songs.up();
                    }
                }
                Mode::Popup => (),
            }
        }
    }

    fn down(&mut self) {
        if !self.delete {
            match self.mode {
                Mode::Playlist => {
                    self.playlists.down();
                }
                Mode::Song => {
                    if let Some(selected) = self.playlists.selected_mut() {
                        selected.songs.down();
                    }
                }
                Mode::Popup => (),
            }
        }
    }

    fn left(&mut self) {
        if self.delete {
            self.yes = true;
        } else if let Mode::Song = self.mode {
            self.mode = Mode::Playlist;
        }
    }

    fn right(&mut self) {
        if self.delete {
            self.yes = false;
        } else {
            match self.mode {
                Mode::Playlist if self.playlists.selected().is_some() => self.mode = Mode::Song,
                _ => (),
            }
        }
    }
}

pub fn on_enter(playlist: &mut Playlist, player: &mut Player) {
    //No was selected by the user.
    if playlist.delete && !playlist.yes {
        playlist.yes = true;
        return playlist.delete = false;
    }

    match playlist.mode {
        Mode::Playlist if playlist.delete => delete_playlist(playlist),
        Mode::Song if playlist.delete => delete_song(playlist),
        Mode::Playlist => {
            if let Some(selected) = playlist.playlists.selected() {
                let songs: Vec<Song> = selected
                    .songs
                    .data
                    .iter()
                    .map(|song| Song::from(&song.into_bytes(), 0))
                    .collect();

                match player.add_songs(&songs) {
                    Ok(_) => (),
                    Err(e) => log!("{}", e),
                }
                save_queue(player);
            }
        }
        Mode::Song => {
            if let Some(selected) = playlist.playlists.selected() {
                if let Some(song) = selected.songs.selected() {
                    match player.add_songs(&[Song::from(&song.into_bytes(), 0)]) {
                        Ok(_) => (),
                        Err(e) => log!("{}", e),
                    }
                    save_queue(player);
                }
            }
        }
        Mode::Popup if !playlist.song_buffer.is_empty() => {
            let name = playlist.search_query.trim().to_string();
            let pos = playlist.playlists.data.iter().position(|p| p.name == name);
            let songs: Vec<RawSong> = playlist.song_buffer.iter().map(RawSong::from).collect();

            if let Some(pos) = pos {
                let p = &mut playlist.playlists.data[pos];
                p.songs.data.extend(songs);
                p.songs.select(Some(0));
                p.save();
                playlist.playlists.select(Some(pos));
            } else {
                let len = playlist.playlists.len();
                playlist.playlists.data.push(RawPlaylist::new(&name, songs));
                playlist.playlists.select(Some(len));
                playlist.playlists.data[len].save();
            }

            //Reset everything.
            playlist.search_query = String::new();
            playlist.mode = Mode::Playlist;
        }
        Mode::Popup => (),
    }
}

pub fn on_backspace(playlist: &mut Playlist, control: bool) {
    match playlist.mode {
        Mode::Popup => {
            playlist.changed = true;
            if control {
                playlist.search_query.clear();
            } else {
                playlist.search_query.pop();
            }
        }
        _ => playlist.left(),
    }
}

pub fn add(playlist: &mut Playlist, songs: &[Song]) {
    playlist.song_buffer = songs.to_vec();
    playlist.mode = Mode::Popup;
}

fn delete_song(playlist: &mut Playlist) {
    if let Some(i) = playlist.playlists.index() {
        let selected = &mut playlist.playlists.data[i];

        if let Some(j) = selected.songs.index() {
            selected.songs.remove(j);
            selected.save();

            //If there are no songs left delete the playlist.
            if selected.songs.is_empty() {
                gonk_database::remove_playlist(&selected.path);
                playlist.playlists.remove(i);
            }
        }
        playlist.delete = false;
    }
}

fn delete_playlist(playlist: &mut Playlist) {
    if let Some(index) = playlist.playlists.index() {
        gonk_database::remove_playlist(&playlist.playlists.data[index].path);
        playlist.playlists.remove(index);
        playlist.delete = false;
    }
}

pub fn delete(playlist: &mut Playlist, shift: bool) {
    match playlist.mode {
        Mode::Playlist if shift => delete_playlist(playlist),
        Mode::Song if shift => delete_song(playlist),
        Mode::Playlist | Mode::Song => {
            playlist.delete = true;
        }
        Mode::Popup => (),
    }
}

pub fn on_escape(playlist: &mut Playlist) {
    if playlist.delete {
        playlist.yes = true;
        playlist.delete = false;
    } else if let Mode::Popup = playlist.mode {
        playlist.mode = Mode::Playlist;
        playlist.search_query = String::new();
        playlist.changed = true;
    }
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
        let len = playlist.search_query.len() as u16;
        let width = v[0].width.saturating_sub(1);
        let offset_x = if len < width { 0 } else { len - width + 1 };

        f.render_widget(
            Paragraph::new(playlist.search_query.as_str())
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
            let eq = playlist
                .playlists
                .data
                .iter()
                .any(|p| p.name == playlist.search_query);
            playlist.search_result = if eq {
                format!("Add to existing playlist: {}", playlist.search_query)
            } else if playlist.search_query.is_empty() {
                String::from("Enter a playlist name...")
            } else {
                format!("Add to new playlist: {}", playlist.search_query)
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
        if playlist.search_query.is_empty() {
            f.set_cursor(x, y);
        } else {
            let width = v[0].width.saturating_sub(3);
            if len < width {
                f.set_cursor(x + len, y);
            } else {
                f.set_cursor(x + width, y);
            }
        }
    }
}

pub fn draw_delete_popup(playlist: &mut Playlist, f: &mut Frame) {
    if let Some(area) = centered_rect(20, 6, f.size()) {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Percentage(90)])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(v[1]);

        let (yes, no) = if playlist.yes {
            (
                Style::default(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            )
        } else {
            (
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
                Style::default(),
            )
        };

        f.render_widget(Clear, area);

        let delete_msg = if let Mode::Playlist = playlist.mode {
            "Delete playlist?"
        } else {
            "Delete song?"
        };

        f.render_widget(
            Paragraph::new(delete_msg)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .alignment(Alignment::Center),
            v[0],
        );

        f.render_widget(
            Paragraph::new("Yes")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(yes)
                        .border_type(BorderType::Rounded),
                )
                .style(yes)
                .alignment(Alignment::Center),
            horizontal[0],
        );

        f.render_widget(
            Paragraph::new("No")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(no)
                        .border_type(BorderType::Rounded),
                )
                .style(no)
                .alignment(Alignment::Center),
            horizontal[1],
        );
    }
}

pub fn draw(playlist: &mut Playlist, area: Rect, f: &mut Frame, event: Option<MouseEvent>) {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    if let Some(event) = event {
        let rect = Rect {
            x: event.column,
            y: event.row,
            ..Default::default()
        };
        if rect.intersects(horizontal[1]) {
            playlist.mode = Mode::Song;
        } else if rect.intersects(horizontal[0]) {
            playlist.mode = Mode::Playlist;
        }
    }

    let items: Vec<ListItem> = playlist
        .playlists
        .data
        .iter()
        .map(|p| p.name.clone())
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
        &mut ListState::new(playlist.playlists.index()),
    );

    if let Some(selected) = playlist.playlists.selected() {
        let content: Vec<Row> = selected
            .songs
            .data
            .iter()
            .map(|song| {
                Row::new(vec![
                    Span::styled(song.title(), Style::default().fg(COLORS.title)),
                    Span::styled(song.album(), Style::default().fg(COLORS.album)),
                    Span::styled(song.artist(), Style::default().fg(COLORS.artist)),
                ])
            })
            .collect();

        let table = Table::new(&content)
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
            &mut TableState::new(selected.songs.index()),
        );
    } else {
        f.render_widget(
            Block::default()
                .title("─Songs")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            horizontal[1],
        );
    }

    if playlist.delete {
        draw_delete_popup(playlist, f);
    } else if let Mode::Popup = playlist.mode {
        draw_popup(playlist, f);
    }
}

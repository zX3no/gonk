use crate::{log, save_queue, widgets::*, Frame, Input, COLORS};
use gonk_database::{Index, RawPlaylist, RawSong, Song};
use gonk_player::Player;
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

pub struct Playlist {
    pub mode: Mode,

    ///Contains the playlist names and songs.
    pub playlists: Index<RawPlaylist>,

    ///Stored that we're requested to be added
    pub song_buffer: Vec<Song>,

    ///The users search query.
    pub search_query: String,
    ///The name of the playlist the user will add to.
    pub search_result: String,
    ///Has the search changed?
    pub changed: bool,
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
        }
    }
}

impl Input for Playlist {
    fn up(&mut self) {
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

    fn down(&mut self) {
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

    fn left(&mut self) {
        if self.mode == Mode::Song {
            self.mode = Mode::Playlist;
        }
    }

    fn right(&mut self) {
        match self.playlists.selected() {
            Some(_) if self.mode == Mode::Playlist => {
                self.mode = Mode::Song;
            }
            _ => (),
        }
    }
}

pub fn on_enter(playlist: &mut Playlist, player: &mut Player) {
    match playlist.mode {
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

            match pos {
                //Playlist exists
                Some(pos) => {
                    let p = &mut playlist.playlists.data[pos];
                    p.songs.data.extend(songs);
                    p.songs.select(Some(0));
                    p.save();
                    playlist.playlists.select(Some(pos));
                }
                //Playlist does not exist.
                None => {
                    let len = playlist.playlists.len();
                    playlist.playlists.data.push(RawPlaylist::new(&name, songs));
                    playlist.playlists.select(Some(len));
                    playlist.playlists.data[len].save();
                }
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

pub fn add_to_playlist(playlist: &mut Playlist, songs: &[Song]) {
    playlist.song_buffer = songs.to_vec();
    playlist.mode = Mode::Popup;
}

pub fn delete(playlist: &mut Playlist) {
    match playlist.mode {
        Mode::Playlist => {
            //TODO: Prompt the user with yes or no.
            if let Some(index) = playlist.playlists.index() {
                gonk_database::remove_playlist(&playlist.playlists.data[index].path);
                playlist.playlists.remove(index);
            }
        }
        Mode::Song => {
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
            }
        }
        Mode::Popup => (),
    }
}

pub fn on_escape(playlist: &mut Playlist, mode: &mut super::Mode) {
    match playlist.mode {
        Mode::Popup => {
            playlist.mode = Mode::Playlist;
            playlist.search_query = String::new();
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

//TODO: Prompt the user with yes or no on deletes.
//TODO: Clear playlist with confirmation.
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

pub fn draw(playlist: &mut Playlist, area: Rect, f: &mut Frame) {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

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
                    Span::styled(song.title(), Style::default().fg(COLORS.name)),
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

    if let Mode::Popup = playlist.mode {
        draw_popup(playlist, f);
    }
}

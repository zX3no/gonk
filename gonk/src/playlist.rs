use crate::{ALBUM, ARTIST, TITLE};
use gonk_core::{Index, Song};
use gonk_player::Player;
use std::{error::Error, io::Stdout, mem};
use winter::*;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Playlist,
    Song,
    Popup,
}

pub struct Playlist {
    pub mode: Mode,
    pub lists: Index<gonk_core::Playlist>,
    pub song_buffer: Vec<Song>,
    pub search_query: String,
    pub search_result: String,
    pub changed: bool,
    pub delete: bool,
    pub yes: bool,
}

impl Playlist {
    pub fn new() -> std::result::Result<Self, Box<dyn Error>> {
        Ok(Self {
            mode: Mode::Playlist,
            lists: Index::from(gonk_core::playlist::playlists()),
            song_buffer: Vec::new(),
            changed: false,
            search_query: String::new(),
            search_result: String::from("Enter a playlist name..."),
            delete: false,
            yes: true,
        })
    }
}

pub fn up(playlist: &mut Playlist) {
    if !playlist.delete {
        match playlist.mode {
            Mode::Playlist => {
                playlist.lists.up();
            }
            Mode::Song => {
                if let Some(selected) = playlist.lists.selected_mut() {
                    selected.songs.up();
                }
            }
            Mode::Popup => (),
        }
    }
}

pub fn down(playlist: &mut Playlist) {
    if !playlist.delete {
        match playlist.mode {
            Mode::Playlist => {
                playlist.lists.down();
            }
            Mode::Song => {
                if let Some(selected) = playlist.lists.selected_mut() {
                    selected.songs.down();
                }
            }
            Mode::Popup => (),
        }
    }
}

pub fn left(playlist: &mut Playlist) {
    if playlist.delete {
        playlist.yes = true;
    } else if let Mode::Song = playlist.mode {
        playlist.mode = Mode::Playlist;
    }
}

pub fn right(playlist: &mut Playlist) {
    if playlist.delete {
        playlist.yes = false;
    } else {
        match playlist.mode {
            Mode::Playlist if playlist.lists.selected().is_some() => playlist.mode = Mode::Song,
            _ => (),
        }
    }
}

pub const fn centered_rect(width: u16, height: u16, area: Rect) -> Option<Rect> {
    let v = area.height / 2;
    let h = area.width / 2;
    let mut rect = area.inner((v.saturating_sub(height / 2), h.saturating_sub(width / 2)));
    rect.width = width;
    rect.height = height;
    if area.height < rect.height || area.width < rect.width {
        None
    } else {
        Some(rect)
    }
}

pub fn draw(
    playlist: &mut Playlist,
    area: winter::Rect,
    buf: &mut winter::Buffer,
    mouse: Option<(u16, u16)>,
    stdout: &mut Stdout,
) {
    let horizontal = layout![
        area,
        Direction::Horizontal,
        Constraint::Percentage(30),
        Constraint::Percentage(70)
    ];

    if let Some((x, y)) = mouse {
        let rect = Rect {
            x,
            y,
            ..Default::default()
        };
        if rect.intersects(horizontal[1]) {
            playlist.mode = Mode::Song;
        } else if rect.intersects(horizontal[0]) {
            playlist.mode = Mode::Playlist;
        }
    }

    let items: Vec<_> = playlist.lists.iter().map(|p| lines!(p.name())).collect();
    let symbol = if let Mode::Playlist = playlist.mode {
        ">"
    } else {
        ""
    };
    list(
        Some(block(Some("Playlist".into()), Borders::ALL, Rounded).margin(1)),
        items,
        Some(symbol),
        None,
    )
    .draw(horizontal[0], buf, playlist.lists.index());

    let song_block = block(Some("Songs".into()), Borders::ALL, Rounded).margin(1);
    if let Some(selected) = playlist.lists.selected() {
        let rows: Vec<_> = selected
            .songs
            .iter()
            .map(|song| {
                row![
                    text!(&song.title, fg(TITLE)),
                    text!(&song.album, fg(ALBUM)),
                    text!(&song.artist, fg(ARTIST))
                ]
            })
            .collect();

        let symbol = if playlist.mode == Mode::Playlist {
            ">"
        } else {
            ""
        };
        table(
            None,
            Some(song_block),
            &[
                Constraint::Percentage(42),
                Constraint::Percentage(30),
                Constraint::Percentage(28),
            ],
            rows,
            Some(symbol),
            style(),
        )
        .draw(horizontal[1], buf, selected.songs.index());
    } else {
        song_block.draw(horizontal[1], buf);
    }

    if playlist.delete {
        if let Some(area) = centered_rect(20, 5, area) {
            let v = layout![
                area,
                Direction::Vertical,
                Constraint::Length(3),
                Constraint::Percentage(90)
            ];
            let h = layout![
                v[1],
                Direction::Horizontal,
                Constraint::Percentage(50),
                Constraint::Percentage(50)
            ];

            let (yes, no) = if playlist.yes {
                //TODO: This was DarkGray. I don't know what it's suppose to e.
                (underlined(), fg(Black).dim())
            } else {
                (fg(Black).dim().underlined(), underlined())
            };

            let delete_msg = if let Mode::Playlist = playlist.mode {
                "Delete playlist?"
            } else {
                "Delete song?"
            };

            //TODO: Clear the area.
            // f.render_widget(Clear, area);

            lines!(delete_msg)
                .block(None, Borders::TOP | Borders::LEFT | Borders::RIGHT, Rounded)
                .align(Center)
                .draw(v[0], buf);

            lines_s!("Yes", yes)
                .block(None, Borders::LEFT | Borders::BOTTOM, Rounded)
                .align(Center)
                .draw(h[0], buf);

            lines_s!("No", no)
                .block(None, Borders::RIGHT | Borders::BOTTOM, Rounded)
                .align(Center)
                .draw(h[1], buf);
        }
    } else if let Mode::Popup = playlist.mode {
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
        if let Some(area) = centered_rect(45, 6, area) {
            let v = layout(
                area,
                Direction::Vertical,
                (1, 1),
                [Constraint::Length(3), Constraint::Percentage(50)].into(),
            );
            //TODO: Fix margin in macro.
            // let v = layout![
            //     area,
            //     Direction::Vertical,
            //     (1, 1),
            //     Constraint::Length(3),
            //     Constraint::Percentage(50)
            // ];

            //TODO: Clear area.
            // f.render_widget(Clear, area);

            block(Some("Add to playlist".into()), ALL, Rounded)
                .margin(1)
                .draw(area, buf);

            //Scroll the playlist name.
            let len = playlist.search_query.len() as u16;
            let width = v[0].width.saturating_sub(1);
            let offset_x = if len < width { 0 } else { len - width + 1 };

            //TODO: Scroll.
            lines!(playlist.search_query.as_str())
                .block(None, ALL, Rounded)
                .draw(v[0], buf);
            // f.render_widget(
            //     Paragraph::new(playlist.search_query.as_str())
            //         .block(Block::default().borders(Borders::ALL).border_type(Rounded))
            //         .scroll((0, offset_x)),
            //     v[0],
            // );

            //TODO: Underline `new` and `existing` to clarify what is happening.
            if playlist.changed {
                playlist.changed = false;
                let eq = playlist
                    .lists
                    .iter()
                    .any(|p| p.name() == playlist.search_query);
                playlist.search_result = if eq {
                    format!("Add to existing playlist: {}", playlist.search_query)
                } else if playlist.search_query.is_empty() {
                    String::from("Enter a playlist name...")
                } else {
                    format!("Add to new playlist: {}", playlist.search_query)
                }
            }

            lines!(playlist.search_result.as_str()).draw(v[1].inner((1, 0)), buf);

            // f.render_widget(
            //     Paragraph::new(playlist.search_result.as_str()),
            //     v[1].inner(&Margin {
            //         horizontal: 1,
            //         vertical: 0,
            //     }),
            // );

            //Draw the cursor.
            //TODO: Is move_to the same as set_cursor?
            //TODO: Need to deal with hiding and unhiding the curosr.
            let (x, y) = (v[0].x + 1, v[0].y + 1);
            if playlist.search_query.is_empty() {
                // f.set_cursor(x, y);
            } else {
                let width = v[0].width.saturating_sub(3);
                if len < width {
                    // f.set_cursor(x + len, y);
                } else {
                    // f.set_cursor(x + width, y);
                }
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
            if let Some(selected) = playlist.lists.selected() {
                player.add(selected.songs.clone());
            }
        }
        Mode::Song => {
            if let Some(selected) = playlist.lists.selected() {
                if let Some(song) = selected.songs.selected() {
                    player.add(vec![song.clone()]);
                }
            }
        }
        Mode::Popup if !playlist.song_buffer.is_empty() => {
            //Find the index of the playlist
            let name = playlist.search_query.trim().to_string();
            let pos = playlist.lists.iter().position(|p| p.name() == name);

            let songs = mem::take(&mut playlist.song_buffer);

            //If the playlist exists
            if let Some(pos) = pos {
                let pl = &mut playlist.lists[pos];
                pl.songs.extend(songs);
                pl.songs.select(Some(0));
                pl.save().unwrap();
                playlist.lists.select(Some(pos));
            } else {
                //If the playlist does not exist create it.
                let len = playlist.lists.len();
                playlist.lists.push(gonk_core::Playlist::new(&name, songs));
                playlist.lists[len].save().unwrap();
                playlist.lists.select(Some(len));
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
        _ => left(playlist),
    }
}

pub fn add(playlist: &mut Playlist, songs: Vec<Song>) {
    playlist.song_buffer = songs;
    playlist.mode = Mode::Popup;
}

fn delete_song(playlist: &mut Playlist) {
    if let Some(i) = playlist.lists.index() {
        let selected = &mut playlist.lists[i];

        if let Some(j) = selected.songs.index() {
            selected.songs.remove(j);
            selected.save().unwrap();

            //If there are no songs left delete the playlist.
            if selected.songs.is_empty() {
                selected.delete().unwrap();
                playlist.lists.remove_and_move(i);
                playlist.mode = Mode::Playlist;
            }
        }
        playlist.delete = false;
    }
}

fn delete_playlist(playlist: &mut Playlist) {
    if let Some(index) = playlist.lists.index() {
        // playlist.playlists[index].delete();
        playlist.lists.remove_and_move(index);
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

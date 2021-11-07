use std::cmp::Ordering;

use tui::backend::Backend;
use tui::layout::*;
use tui::style::*;
use tui::widgets::*;
use tui::Frame;

use crate::app::{App, BrowserMode, Mode};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    match app.ui_mode {
        Mode::Browser => draw_browser(f, app),
        Mode::Queue => draw_queue(f, app),
    }
}

pub fn draw_browser<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();

    let music = &mut app.music;

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

    let a: Vec<_> = music
        .artist_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let b: Vec<_> = music
        .album_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    //clone is not optional :(
    let c: Vec<_> = music
        .song_names()
        .iter()
        .map(|name| ListItem::new(name.clone()))
        .collect();

    let artists = List::new(a)
        .block(
            Block::default()
                .title("Aritst")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol(">");

    let mut artist_state = ListState::default();
    artist_state.select(music.get_selected_artist());

    let albums = List::new(b)
        .block(
            Block::default()
                .title("Album")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol(">");

    let mut album_state = ListState::default();
    album_state.select(music.get_selected_album());

    let songs = List::new(c)
        .block(
            Block::default()
                .title("Song")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol(">");

    let mut song_state = ListState::default();
    song_state.select(music.get_selected_song());

    match app.browser_mode {
        BrowserMode::Artist => {
            album_state = ListState::default();
            song_state = ListState::default();
        }
        BrowserMode::Album => {
            artist_state = ListState::default();
            song_state = ListState::default();
        }
        BrowserMode::Song => {
            artist_state = ListState::default();
            album_state = ListState::default();
        }
    }

    f.render_stateful_widget(artists, chunks[0], &mut artist_state);
    f.render_stateful_widget(albums, chunks[1], &mut album_state);
    f.render_stateful_widget(songs, chunks[2], &mut song_state);
}

//TODO: store the duration in the database
pub fn draw_queue<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();

    let (songs, index, ui_index) = (
        &app.queue.songs,
        &app.queue.now_playing,
        &app.queue.ui_index,
    );

    let widths = if !songs.is_empty() {
        // let number: Vec<String> = songs
        //     .into_iter()
        //     .map(|song| song.number.to_string())
        //     .collect();
        let name: Vec<&String> = songs.into_iter().map(|song| &song.name).collect();
        let album: Vec<&String> = songs.into_iter().map(|song| &song.album).collect();
        let artist: Vec<&String> = songs.into_iter().map(|song| &song.artist).collect();

        // let c1 = find_max_owned(&number);
        let c1 = 4.0;
        let c2 = find_max(&name) as f32;
        let c3 = find_max(&album) as f32;
        let c4 = find_max(&artist) as f32;
        let total = c1 + c2 + c3 + c4;

        let c1 = c1 / total * 100.0;
        let c2 = c2 / total * 100.0;
        let c3 = c3 / total * 100.0;
        let c4 = c4 / total * 100.0;

        let test = largest_remainder(&[c1, c2, c3, c4]);

        let c1 = *test.get(0).unwrap() as u16;
        let c2 = *test.get(1).unwrap() as u16;
        let c3 = *test.get(2).unwrap() as u16;
        let c4 = *test.get(3).unwrap() as u16;
        // dbg!(c1, c2, c3, c4);
        // panic!();

        [
            Constraint::Percentage(c1),
            Constraint::Percentage(c2),
            Constraint::Percentage(c3),
            Constraint::Percentage(c4),
        ]
    } else {
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]
    };

    let mut items: Vec<Row> = songs
        .iter()
        .map(|song| {
            Row::new(vec![
                Cell::from(song.number.to_string()).style(Style::default().fg(Color::Green)),
                Cell::from(song.name.to_owned()).style(Style::default().fg(Color::Cyan)),
                Cell::from(song.album.to_owned()).style(Style::default().fg(Color::Magenta)),
                Cell::from(song.artist.to_owned()).style(Style::default().fg(Color::Blue)),
            ])
        })
        .collect();

    if let Some(index) = index {
        if let Some(song) = songs.get(*index) {
            if let Some(other_index) = ui_index {
                //ui selection and now_playing match
                let row = if index == other_index {
                    Row::new(vec![
                        Cell::from(song.number.to_string())
                            .style(Style::default().bg(Color::Green)),
                        Cell::from(song.name.to_owned()).style(Style::default().bg(Color::Cyan)),
                        Cell::from(song.album.to_owned())
                            .style(Style::default().bg(Color::Magenta)),
                        Cell::from(song.artist.to_owned()).style(Style::default().bg(Color::Blue)),
                    ])
                    .style(Style::default().fg(Color::Black))
                } else {
                    Row::new(vec![
                        Cell::from(song.number.to_string())
                            .style(Style::default().fg(Color::Green)),
                        Cell::from(song.name.to_owned()).style(Style::default().fg(Color::Cyan)),
                        Cell::from(song.album.to_owned())
                            .style(Style::default().fg(Color::Magenta)),
                        Cell::from(song.artist.to_owned()).style(Style::default().fg(Color::Blue)),
                    ])
                    .style(
                        Style::default()
                            .fg(Color::Black)
                            .add_modifier(Modifier::ITALIC),
                    )
                };
                items.remove(*index);
                items.insert(*index, row);

                if let Some(other_song) = songs.get(*other_index) {
                    let other_row = Row::new(vec![
                        Cell::from(other_song.number.to_string())
                            .style(Style::default().bg(Color::Green)),
                        Cell::from(other_song.name.to_owned())
                            .style(Style::default().bg(Color::Cyan)),
                        Cell::from(other_song.album.to_owned())
                            .style(Style::default().bg(Color::Magenta)),
                        Cell::from(other_song.artist.to_owned())
                            .style(Style::default().bg(Color::Blue)),
                    ])
                    .style(Style::default().fg(Color::Black));

                    items.remove(*other_index);
                    items.insert(*other_index, other_row);
                }
            }
        }
    }

    let t = Table::new(items)
        .header(
            Row::new(vec!["Track", "Title", "Album", "Artist"])
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .widths(&widths)
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol("> ");

    //TODO: calculate longest length of track, album, artist name and change the constraints to fit
    //sometimes the track name is squished when it doesn't need too

    let mut state = TableState::default();
    state.select(*index);
    f.render_stateful_widget(t, area, &mut state);
}
fn find_max(items: &Vec<&String>) -> usize {
    let mut i = 0;
    for item in items {
        if item.len() > i {
            i = item.len();
        }
    }
    i
}
fn find_max_owned(items: &Vec<String>) -> f32 {
    let mut i = 0;
    for item in items {
        if item.len() > i {
            i = item.len();
        }
    }
    i as f32
}
fn largest_remainder(numbers: &[f32]) -> Vec<f32> {
    //trunc | rem
    let mut nums = Vec::new();
    for (i, num) in numbers.iter().enumerate() {
        nums.push((num.trunc(), num - (num / 1.0).trunc() * 1.0, i));
    }

    nums.sort_by(|(_, a, _), (_, b, _)| a.partial_cmp(b).unwrap());

    let mut out: Vec<f32> = nums.iter().map(|(trunc, _, _)| trunc.clone()).collect();
    while out.iter().sum::<f32>() != 100.0 {
        let (trunc, rem, _) = nums.last().unwrap().clone();

        let mut index = None;
        let mut order = None;
        for (i, (_, r, o)) in nums.iter().enumerate() {
            if rem == *r {
                index = Some(i);
                order = Some(*o);
            }
        }
        nums.remove(index.unwrap());
        nums.insert(index.unwrap(), (trunc + 1.0, 0.0, order.unwrap()));

        out = nums.iter().map(|(trunc, _, _)| trunc.clone()).collect();
    }

    nums.sort_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap());

    nums.into_iter().map(|(trunc, _, _)| trunc).collect()
}

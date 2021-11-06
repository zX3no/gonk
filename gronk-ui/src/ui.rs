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
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
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

//Number - Name - Album - Artist - Duration
//TODO: store the duration in the database
pub fn draw_queue<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();

    let queue = app.music.get_queue();

    let mut items: Vec<Row> = queue
        .songs
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

    if let Some(index) = queue.index {
        if let Some(song) = queue.songs.get(index) {
            let row = Row::new(vec![
                Cell::from(song.number.to_string()).style(Style::default().bg(Color::Green)),
                Cell::from(song.name.to_owned()).style(Style::default().bg(Color::Cyan)),
                Cell::from(song.album.to_owned()).style(Style::default().bg(Color::Magenta)),
                Cell::from(song.artist.to_owned()).style(Style::default().bg(Color::Blue)),
            ])
            //make the text black
            .style(Style::default().fg(Color::Black));
            //i don't think this will realocate? need to test..
            items.remove(index);
            items.insert(index, row);
        }
    }

    let t = Table::new(items)
        // You can set the style of the entire Table.
        .style(Style::default().fg(Color::White))
        // It has an optional header, which is simply a Row always visible at the top.
        .header(
            Row::new(vec!["Track", "Title", "Album", "Artist"])
                .style(Style::default().fg(Color::White))
                // If you want some space between the header and the rest of the rows, you can always
                // specify some margin at the bottom.
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .widths(&[
            Constraint::Min(5),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ]);

    let mut state = TableState::default();
    state.select(queue.index);
    f.render_stateful_widget(t, area, &mut state);
}

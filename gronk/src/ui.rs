use tui::backend::Backend;
use tui::layout::*;
use tui::style::*;
use tui::text::*;
use tui::widgets::*;
use tui::Frame;

use crate::app::App;
use crate::modes::{BrowserMode, UiMode};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    match app.ui_mode {
        UiMode::Browser => draw_browser(f, app),
        UiMode::Queue => draw_queue(f, app),
        UiMode::Search => draw_search(f, app),
    }
}
pub fn draw_search<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Percentage(90)].as_ref())
        .split(area);

    let p = Paragraph::new(vec![Spans::from(app.search.query.as_str())])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);

    let items: Vec<Row> = app
        .get_search()
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
        .widths(&[
            Constraint::Length(6),
            Constraint::Percentage(40),
            Constraint::Percentage(25),
            Constraint::Percentage(100),
        ])
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol("> ");

    f.render_widget(p, chunks[0]);
    f.render_widget(t, chunks[1]);

    if app.search.query.is_empty() {
        f.set_cursor(1, 1);
    } else {
        let mut len = app.search.query.len() as u16;
        //does this even work?
        if len > area.width {
            len = area.width;
        }
        f.set_cursor(len + 1, 1);
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
    let con = [
        Constraint::Percentage(app.constraint[0]),
        Constraint::Percentage(app.constraint[1]),
        Constraint::Percentage(app.constraint[2]),
        Constraint::Percentage(app.constraint[3]),
    ];

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
        //TODO: change number to length(6)
        .widths(&con)
        // ...and potentially show a symbol in front of the selection.
        .highlight_symbol("> ");

    //TODO: calculate longest length of track, album, artist name and change the constraints to fit
    //sometimes the track name is squished when it doesn't need too

    let mut state = TableState::default();
    state.select(*index);
    f.render_stateful_widget(t, area, &mut state);
}

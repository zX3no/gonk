use gronk_search::ItemType;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{
    Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState,
};
use tui::Frame;

use crate::app::App;
use crate::modes::{BrowserMode, UiMode};

static TRACK: Color = Color::Green;
static TITLE: Color = Color::Cyan;
static ALBUM: Color = Color::Magenta;
static ARTIST: Color = Color::Blue;

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

    let p = Paragraph::new(vec![Spans::from(app.search.get_query())])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);

    let results = app.get_search();

    if let Some(db) = &app.database {
        let items = results.iter().map(|r| match r.item_type {
            ItemType::Song => {
                let song = db.get_song_from_id(r.song_id.unwrap());
                Row::new(vec![
                    Cell::from(song.name.to_owned()).style(Style::default().fg(TITLE)),
                    Cell::from(song.album.to_owned()).style(Style::default().fg(ALBUM)),
                    Cell::from(song.artist).style(Style::default().fg(ARTIST)),
                ])
            }
            ItemType::Album => Row::new(vec![
                Cell::from(r.name.to_owned() + " (album)").style(Style::default().fg(TITLE)),
                Cell::from("").style(Style::default().fg(ALBUM)),
                Cell::from(r.album_artist.as_ref().unwrap().clone())
                    .style(Style::default().fg(ARTIST)),
            ]),
            ItemType::Artist => Row::new(vec![
                Cell::from(r.name.to_owned() + " (artist)").style(Style::default().fg(TITLE)),
                Cell::from("").style(Style::default().fg(ALBUM)),
                Cell::from("").style(Style::default().fg(ARTIST)),
            ]),
        });

        let t = Table::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .widths(&[
                Constraint::Percentage(43),
                Constraint::Percentage(29),
                Constraint::Percentage(27),
            ])
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol("> ");

        let mut state = TableState::default();
        state.select(app.search.state());

        f.render_widget(p, chunks[0]);
        f.render_stateful_widget(t, chunks[1], &mut state);

        if app.search.show_cursor() {
            if app.search.empty_cursor() {
                f.set_cursor(1, 1);
            } else {
                let mut len = app.search.query_len();
                //does this even work?
                if len > area.width {
                    len = area.width;
                }
                f.set_cursor(len + 1, 1);
            }
        }
    }
}
pub fn draw_browser<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();

    let music = &mut app.browser;

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
//abstract selection color into it's own widget
pub fn draw_queue<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(20),
            Constraint::Length(2),
        ])
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_songs(f, app, chunks[1]);
    draw_seeker(f, app, chunks[2]);
}

pub fn draw_header<B: Backend>(f: &mut Frame<B>, app: &mut App, chunk: Rect) {
    let playback = if app.queue.is_empty() {
        "[stopped]"
    } else if app.queue.is_playing() {
        "[playing]"
    } else {
        "[paused]"
    };

    let time = if let Some(duration) = app.queue.duration() {
        let elapsed = app.queue.elapsed().as_secs_f64();
        let duration = duration.as_secs_f64();

        let mins = elapsed / 60.0;
        let rem = elapsed % 60.0;
        let e = format!(
            "{:0width$}:{:0width$}",
            mins.trunc() as usize,
            rem.trunc() as usize,
            width = 2,
        );

        let mins = duration / 60.0;
        let rem = duration % 60.0;
        let d = format!(
            "{:0width$}:{:0width$}",
            mins.trunc() as usize,
            rem.trunc() as usize,
            width = 2,
        );

        format!("{}/{}", e, d)
    } else {
        String::from("0:00/0:00")
    };

    let left = vec![Spans::from(time), Spans::from(playback)];

    let spacer: String = {
        let width = chunk.width;
        (0..width - 4).map(|_| "-").collect()
    };

    let center = if let Some(song) = app.queue.get_playing() {
        vec![
            Spans::from(vec![
                Span::raw("--| "),
                Span::styled(&song.artist, Style::default().fg(ARTIST)),
                Span::raw(" - "),
                Span::styled(&song.name, Style::default().fg(TITLE)),
                Span::raw(" |--"),
            ]),
            Spans::from(Span::styled(&song.album, Style::default().fg(ALBUM))),
            Spans::from(""),
            Spans::from(spacer),
        ]
    } else {
        vec![
            Spans::default(),
            Spans::default(),
            Spans::default(),
            Spans::from(spacer),
        ]
    };

    let volume = app.queue.get_volume_percent();

    let right = vec![Spans::from(vec![
        Span::raw(format!("Vol: {}%", volume)),
        //TODO: what does hidden even do?
        //this does align properly
        Span::styled("ㅤ", Style::default().add_modifier(Modifier::HIDDEN)),
    ])];

    let b = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_type(BorderType::Rounded);

    let left = Paragraph::new(left)
        .block(b.clone())
        .alignment(Alignment::Left);

    let center = Paragraph::new(center)
        .block(b.clone())
        .alignment(Alignment::Center);

    let right = Paragraph::new(right).block(b).alignment(Alignment::Right);

    let chunk = Rect {
        x: chunk.x,
        y: chunk.y,
        width: chunk.width,
        height: chunk.height,
    };

    f.render_widget(left, chunk);
    f.render_widget(center, chunk);
    f.render_widget(right, chunk);
}

pub fn draw_songs<B: Backend>(f: &mut Frame<B>, app: &mut App, chunk: Rect) {
    let (songs, now_playing, ui_index) = (
        &app.queue.list.songs,
        &app.queue.list.now_playing,
        &app.queue.ui_index.index,
    );

    let mut items: Vec<Row> = songs
        .iter()
        .map(|song| {
            Row::new(vec![
                Cell::from(""),
                Cell::from(song.number.to_string()).style(Style::default().fg(TRACK)),
                Cell::from(song.name.to_owned()).style(Style::default().fg(TITLE)),
                Cell::from(song.album.to_owned()).style(Style::default().fg(ALBUM)),
                Cell::from(song.artist.to_owned()).style(Style::default().fg(ARTIST)),
            ])
        })
        .collect();

    if let Some(playing_index) = now_playing {
        if let Some(song) = songs.get(*playing_index) {
            if let Some(ui_index) = ui_index {
                //currently playing song
                let row = if ui_index == playing_index {
                    Row::new(vec![
                        Cell::from(">>").style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::DIM | Modifier::BOLD),
                        ),
                        Cell::from(song.number.to_string())
                            .style(Style::default().bg(TRACK).fg(Color::Black)),
                        Cell::from(song.name.to_owned())
                            .style(Style::default().bg(TITLE).fg(Color::Black)),
                        Cell::from(song.album.to_owned())
                            .style(Style::default().bg(ALBUM).fg(Color::Black)),
                        Cell::from(song.artist.to_owned())
                            .style(Style::default().bg(ARTIST).fg(Color::Black)),
                    ])
                } else {
                    Row::new(vec![
                        Cell::from(">>").style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::DIM | Modifier::BOLD),
                        ),
                        Cell::from(song.number.to_string()).style(Style::default().fg(TRACK)),
                        Cell::from(song.name.to_owned()).style(Style::default().fg(TITLE)),
                        Cell::from(song.album.to_owned()).style(Style::default().fg(ALBUM)),
                        Cell::from(song.artist.to_owned()).style(Style::default().fg(ARTIST)),
                    ])
                };
                items.remove(*playing_index);
                items.insert(*playing_index, row);

                //current selection
                if ui_index != playing_index {
                    let song = songs.get(*ui_index).unwrap();
                    let row = Row::new(vec![
                        Cell::from(""),
                        Cell::from(song.number.to_string()).style(Style::default().bg(TRACK)),
                        Cell::from(song.name.to_owned()).style(Style::default().bg(TITLE)),
                        Cell::from(song.album.to_owned()).style(Style::default().bg(ALBUM)),
                        Cell::from(song.artist.to_owned()).style(Style::default().bg(ARTIST)),
                    ])
                    .style(Style::default().fg(Color::Black));
                    items.remove(*ui_index);
                    items.insert(*ui_index, row);
                }
            }
        }
    }

    let con = [
        Constraint::Length(2),
        Constraint::Percentage(app.constraint[0]),
        Constraint::Percentage(app.constraint[1]),
        Constraint::Percentage(app.constraint[2]),
        Constraint::Percentage(app.constraint[3]),
    ];

    let t = Table::new(items)
        .header(
            Row::new(vec!["", "Track", "Title", "Album", "Artist"])
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded),
        )
        .widths(&con);

    f.render_widget(t, chunk);
}
pub fn draw_seeker<B: Backend>(f: &mut Frame<B>, app: &mut App, chunk: Rect) {
    if app.queue.is_empty() {
        return f.render_widget(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded),
            chunk,
        );
    }

    if let Some((column, row)) = app.clicked_pos {
        let size = f.size();
        //row = 28
        //height = 30
        if size.height - 3 == row || size.height - 2 == row || size.height - 1 == row {
            let ratio = (column - 4) as f64 / size.width as f64;
            let duration = app.queue.duration().unwrap().as_secs_f64();

            let new_time = duration * ratio;
            app.queue.seek_to(new_time);
            app.queue.play();
        }
        app.clicked_pos = None;
    }

    let area = f.size();
    let width = area.width;
    let percent = app.seeker;
    let pos = (width as f64 * percent).ceil() as usize;

    let mut string = String::new();
    for i in 0..(width - 6) {
        if (i as usize) < pos {
            string.push('=');
        } else {
            string.push('-');
        }
    }

    //place the seeker location
    if pos < string.len() - 1 {
        string.remove(pos);
        string.insert(pos, '>');
    } else {
        string.pop();
        string.push('>');
    }

    let p = Paragraph::new(string).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded),
    );

    f.render_widget(p, chunk)
}

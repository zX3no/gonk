use crate::app::{browser::BrowserMode, Browser};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Spans,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, browser: &Browser) {
    draw_browser(f, browser);

    if browser.is_busy() {
        draw_popup(f);
    }
}

pub fn draw_browser<B: Backend>(f: &mut Frame<B>, browser: &Browser) {
    let area = f.size();

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

    let a: Vec<_> = browser
        .artist_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let b: Vec<_> = browser
        .album_names()
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    //clone is not optional :(
    let c: Vec<_> = browser
        .song_names()
        .iter()
        .map(|name| ListItem::new(name.clone()))
        .collect();

    let artists = List::new(a)
        .block(
            Block::default()
                .title("─Aritst")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol(">");

    let mut artist_state = ListState::default();
    artist_state.select(browser.get_selected_artist());

    let albums = List::new(b)
        .block(
            Block::default()
                .title("─Album")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol(">");

    let mut album_state = ListState::default();
    album_state.select(browser.get_selected_album());

    let songs = List::new(c)
        .block(
            Block::default()
                .title("─Song")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol(">");

    let mut song_state = ListState::default();
    song_state.select(browser.get_selected_song());

    //TODO: better way of doing this?
    match browser.mode {
        BrowserMode::Artist => {
            album_state.select(None);
            song_state.select(None);
        }
        BrowserMode::Album => {
            artist_state.select(None);
            song_state.select(None);
        }
        BrowserMode::Song => {
            artist_state.select(None);
            album_state.select(None);
        }
    }

    f.render_stateful_widget(artists, chunks[0], &mut artist_state);
    f.render_stateful_widget(albums, chunks[1], &mut album_state);
    f.render_stateful_widget(songs, chunks[2], &mut song_state);
}

pub fn draw_popup<B: Backend>(f: &mut Frame<B>) {
    let mut area = f.size();

    if (area.width / 2) < 14 || (area.height / 2) < 3 {
        return;
    }

    area.x = (area.width / 2) - 7;
    if (area.width / 2) % 2 == 0 {
        area.y = (area.height / 2) - 3;
    } else {
        area.y = (area.height / 2) - 2;
    }
    area.width = 14;
    area.height = 3;

    let text = vec![Spans::from("Scanning...")];

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Center);

    f.render_widget(p, area);
}

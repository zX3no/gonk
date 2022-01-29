use crate::app::Search;
use crate::ui::{ALBUM, ARTIST, TITLE};
use gonk_database::Database;
use gonk_search::ItemType;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::Spans,
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, search: &Search, db: &Database) {
    let area = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Percentage(90)].as_ref())
        .split(area);

    let p = Paragraph::new(vec![Spans::from(search.get_query())])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);

    let results = search.results();

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
            Cell::from(r.album_artist.as_ref().unwrap().clone()).style(Style::default().fg(ARTIST)),
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
    state.select(search.selected());

    f.render_widget(p, chunks[0]);
    f.render_stateful_widget(t, chunks[1], &mut state);

    if search.show_cursor() {
        if search.empty_cursor() {
            f.set_cursor(1, 1);
        } else {
            let mut len = search.query_len();
            //does this even work?
            if len > area.width {
                len = area.width;
            }
            f.set_cursor(len + 1, 1);
        }
    }
}

use crate::app::Queue;
use gonk_database::Colors;
use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState};
use tui::Frame;

pub fn draw<B: Backend>(f: &mut Frame<B>, queue: &Queue, colors: &Colors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(f.size());

    draw_header(f, queue, chunks[0], colors);
    draw_songs(f, queue, chunks[1], colors);
    draw_seeker(f, queue, chunks[2]);
}

pub fn draw_header<B: Backend>(f: &mut Frame<B>, queue: &Queue, chunk: Rect, colors: &Colors) {
    //Render the borders first
    let b = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_type(BorderType::Rounded);
    f.render_widget(b, chunk);

    //Left
    {
        let time = if queue.is_empty() {
            String::from("╭─Stopped")
        } else if queue.is_playing() {
            if let Some(duration) = queue.duration() {
                let elapsed = queue.elapsed().as_secs_f64();

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

                format!("╭─{}/{}", e, d)
            } else {
                String::from("╭─0:00/0:00")
            }
        } else {
            String::from("╭─Paused")
        };

        let left = Paragraph::new(time).alignment(Alignment::Left);

        f.render_widget(left, chunk);
    }
    //Center
    {
        let center = if let Some(song) = queue.get_playing() {
            //I wish that paragraphs had clipping
            //I think constaints do
            //I could render the -| |- on a seperate layer
            //outside of the constraint which might work better?
            let mut name = song.name.clone();
            while (name.len() + song.artist.len() + "─| - |─".len()) > (chunk.width - 40) as usize
            {
                name.pop();
            }
            let name = name.trim_end().to_string();

            vec![
                Spans::from(vec![
                    Span::raw("─| "),
                    Span::styled(&song.artist, Style::default().fg(colors.artist)),
                    Span::raw(" - "),
                    Span::styled(name, Style::default().fg(colors.title)),
                    Span::raw(" |─"),
                ]),
                Spans::from(Span::styled(&song.album, Style::default().fg(colors.album))),
            ]
        } else {
            vec![Spans::default(), Spans::default()]
        };
        let center = Paragraph::new(center).alignment(Alignment::Center);
        f.render_widget(center, chunk);
    }
    //Right
    {
        let volume = queue.get_volume();
        let text = Spans::from(format!("Vol: {}%─╮", volume));
        let right = Paragraph::new(text).alignment(Alignment::Right);
        f.render_widget(right, chunk);
    }
}

pub fn draw_songs<B: Backend>(f: &mut Frame<B>, queue: &Queue, chunk: Rect, colors: &Colors) {
    if queue.is_empty() {
        return f.render_widget(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded),
            chunk,
        );
    }

    let (songs, now_playing, ui_index) = (&queue.list.data, queue.list.index, queue.ui.index);

    let mut items: Vec<Row> = songs
        .iter()
        .map(|song| {
            Row::new(vec![
                Cell::from(""),
                Cell::from(song.number.to_string()).style(Style::default().fg(colors.track)),
                Cell::from(song.name.to_owned()).style(Style::default().fg(colors.title)),
                Cell::from(song.album.to_owned()).style(Style::default().fg(colors.album)),
                Cell::from(song.artist.to_owned()).style(Style::default().fg(colors.artist)),
            ])
        })
        .collect();

    if let Some(playing_index) = now_playing {
        if let Some(song) = songs.get(playing_index) {
            if let Some(ui_index) = ui_index {
                //Currently playing song
                let row = if ui_index == playing_index {
                    Row::new(vec![
                        Cell::from(">>").style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::DIM | Modifier::BOLD),
                        ),
                        Cell::from(song.number.to_string())
                            .style(Style::default().bg(colors.track).fg(Color::Black)),
                        Cell::from(song.name.to_owned())
                            .style(Style::default().bg(colors.title).fg(Color::Black)),
                        Cell::from(song.album.to_owned())
                            .style(Style::default().bg(colors.album).fg(Color::Black)),
                        Cell::from(song.artist.to_owned())
                            .style(Style::default().bg(colors.artist).fg(Color::Black)),
                    ])
                } else {
                    Row::new(vec![
                        Cell::from(">>").style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::DIM | Modifier::BOLD),
                        ),
                        Cell::from(song.number.to_string())
                            .style(Style::default().fg(colors.track)),
                        Cell::from(song.name.to_owned()).style(Style::default().fg(colors.title)),
                        Cell::from(song.album.to_owned()).style(Style::default().fg(colors.album)),
                        Cell::from(song.artist.to_owned())
                            .style(Style::default().fg(colors.artist)),
                    ])
                };

                items.remove(playing_index);
                items.insert(playing_index, row);

                //Current selection
                if ui_index != playing_index {
                    let song = songs.get(ui_index).unwrap();
                    let row = Row::new(vec![
                        Cell::from(""),
                        Cell::from(song.number.to_string())
                            .style(Style::default().bg(colors.track)),
                        Cell::from(song.name.to_owned()).style(Style::default().bg(colors.title)),
                        Cell::from(song.album.to_owned()).style(Style::default().bg(colors.album)),
                        Cell::from(song.artist.to_owned())
                            .style(Style::default().bg(colors.artist)),
                    ])
                    .style(Style::default().fg(Color::Black));
                    items.remove(ui_index);
                    items.insert(ui_index, row);
                }
            }
        }
    }

    let con = [
        Constraint::Length(2),
        Constraint::Percentage(queue.constraint[0]),
        Constraint::Percentage(queue.constraint[1]),
        Constraint::Percentage(queue.constraint[2]),
        Constraint::Percentage(queue.constraint[3]),
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

    //this is so that scrolling works
    let mut state = TableState::default();
    state.select(ui_index);

    f.render_stateful_widget(t, chunk, &mut state);
}
pub fn draw_seeker<B: Backend>(f: &mut Frame<B>, queue: &Queue, chunk: Rect) {
    if queue.is_empty() {
        return f.render_widget(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded),
            chunk,
        );
    }

    let area = f.size();
    let width = area.width;
    let percent = queue.seeker();
    let pos = (width as f64 * percent).ceil() as usize;

    let mut string: String = (0..width - 6)
        .map(|i| if (i as usize) < pos { '=' } else { '-' })
        .collect();

    //Place the seeker location
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

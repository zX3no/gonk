use crate::widgets::*;
use crate::*;
use crossterm::event::MouseEvent;
use gonk_player::Player;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use unicode_width::UnicodeWidthStr;

pub struct Queue {
    pub ui: Index<()>,
    pub constraint: [u16; 4],
    pub len: usize,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            ui: Index::new(Vec::new(), Some(0)),
            constraint: [6, 37, 31, 26],
            len: 0,
        }
    }
}

impl Input for Queue {
    fn up(&mut self) {
        self.ui.up_with_len(self.len);
    }

    fn down(&mut self) {
        self.ui.down_with_len(self.len);
    }

    fn left(&mut self) {}

    fn right(&mut self) {}
}

pub fn constraint(queue: &mut Queue, row: usize, shift: bool) {
    if shift && queue.constraint[row] != 0 {
        //Move row back.
        queue.constraint[row + 1] += 1;
        queue.constraint[row] = queue.constraint[row].saturating_sub(1);
    } else if queue.constraint[row + 1] != 0 {
        //Move row forward.
        queue.constraint[row] += 1;
        queue.constraint[row + 1] = queue.constraint[row + 1].saturating_sub(1);
    }

    debug_assert!(
        queue.constraint.iter().sum::<u16>() == 100,
        "Constraint went out of bounds: {:?}",
        queue.constraint
    );
}

pub fn delete(queue: &mut Queue, player: &mut Player) {
    if let Some(i) = queue.ui.index() {
        match player.delete_index(i) {
            Ok(_) => save_queue(player),
            Err(e) => log!("{}", e),
        };
        //make sure the ui index is in sync
        let len = player.songs.len().saturating_sub(1);
        if i > len {
            queue.ui.select(Some(len));
        }
    }
}

pub fn draw(queue: &mut Queue, player: &mut Player, f: &mut Frame, event: Option<MouseEvent>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    draw_header(player, f, chunks[0]);

    let row_bounds = draw_body(queue, player, f, chunks[1]);

    draw_seeker(player, f, chunks[2]);

    //Don't handle mouse input when the queue is empty.
    if player.songs.is_empty() {
        return;
    }

    //Handle mouse input.
    if let Some(event) = event {
        let (x, y) = (event.column, event.row);
        let header_height = 5;

        let size = f.size();

        //Mouse support for the seek bar.
        if (size.height - 3 == y || size.height - 2 == y || size.height - 1 == y)
            && size.height > 15
        {
            let ratio = x as f64 / size.width as f64;
            let duration = player.duration().as_secs_f64();
            player.seek_to(duration * ratio);
        }

        //Mouse support for the queue.
        if let Some((start, _)) = row_bounds {
            //Check if you clicked on the header.
            if y >= header_height {
                let index = (y - header_height) as usize + start;

                //Make sure you didn't click on the seek bar
                //and that the song index exists.
                if index < player.songs.len()
                    && ((size.height < 15 && y < size.height.saturating_sub(1))
                        || y < size.height.saturating_sub(3))
                {
                    queue.ui.select(Some(index));
                }
            }
        }
    }
}

fn draw_header(player: &mut Player, f: &mut Frame, area: Rect) {
    f.render_widget(
        Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded),
        area,
    );

    let state = if player.songs.is_empty() {
        String::from("╭─Stopped")
    } else if player.is_playing() {
        String::from("╭─Playing")
    } else {
        String::from("╭─Paused")
    };

    f.render_widget(Paragraph::new(state).alignment(Alignment::Left), area);

    if !player.songs.is_empty() {
        draw_title(player, f, area);
    }

    let volume = Spans::from(format!("Vol: {}%─╮", player.volume));
    f.render_widget(Paragraph::new(volume).alignment(Alignment::Right), area);
}

fn draw_title(player: &mut Player, f: &mut Frame, area: Rect) {
    let title = if let Some(song) = player.songs.selected() {
        let mut artist = song.artist.trim_end().to_string();
        let mut album = song.album.trim_end().to_string();
        let mut title = song.title.trim_end().to_string();
        let max_width = area.width.saturating_sub(30) as usize;

        while artist.width() + album.width() + "-| - |-".width() > max_width {
            if artist.width() > album.width() {
                artist.pop();
            } else {
                album.pop();
            }
        }

        while title.width() > max_width {
            title.pop();
        }

        let n = title
            .width()
            .saturating_sub(artist.width() + album.width() + 3);
        let rem = n % 2;
        let pad_front = " ".repeat(n / 2);
        let pad_back = " ".repeat(n / 2 + rem);

        vec![
            Spans::from(vec![
                Span::raw(format!("─│ {}", pad_front)),
                Span::styled(artist, Style::default().fg(COLORS.artist)),
                Span::raw(" ─ "),
                Span::styled(album, Style::default().fg(COLORS.album)),
                Span::raw(format!("{} │─", pad_back)),
            ]),
            Spans::from(Span::styled(title, Style::default().fg(COLORS.title))),
        ]
    } else {
        Vec::new()
    };

    f.render_widget(Paragraph::new(title).alignment(Alignment::Center), area);
}

fn draw_body(
    queue: &mut Queue,
    player: &mut Player,
    f: &mut Frame,
    area: Rect,
) -> Option<(usize, usize)> {
    if player.songs.is_empty() && log::message().is_none() {
        f.render_widget(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::LEFT | Borders::RIGHT),
            area,
        );
        return None;
    }

    let (songs, player_index, ui_index) =
        (&player.songs.data, player.songs.index(), queue.ui.index());

    let mut items: Vec<Row> = songs
        .iter()
        .map(|song| {
            Row::new(vec![
                Cell::from(""),
                Cell::from(song.number.to_string()).style(Style::default().fg(COLORS.number)),
                Cell::from(song.title.as_str()).style(Style::default().fg(COLORS.title)),
                Cell::from(song.album.as_str()).style(Style::default().fg(COLORS.album)),
                Cell::from(song.artist.as_str()).style(Style::default().fg(COLORS.artist)),
            ])
        })
        .collect();

    if let Some(player_index) = player_index {
        if let Some(song) = songs.get(player_index) {
            if let Some(ui_index) = ui_index {
                //Currently playing song
                let row = if ui_index == player_index {
                    Row::new(vec![
                        Cell::from(">>").style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::DIM | Modifier::BOLD),
                        ),
                        Cell::from(song.number.to_string())
                            .style(Style::default().bg(COLORS.number).fg(Color::Black)),
                        Cell::from(song.title.as_str())
                            .style(Style::default().bg(COLORS.title).fg(Color::Black)),
                        Cell::from(song.album.as_str())
                            .style(Style::default().bg(COLORS.album).fg(Color::Black)),
                        Cell::from(song.artist.as_str())
                            .style(Style::default().bg(COLORS.artist).fg(Color::Black)),
                    ])
                } else {
                    Row::new(vec![
                        Cell::from(">>").style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::DIM | Modifier::BOLD),
                        ),
                        Cell::from(song.number.to_string())
                            .style(Style::default().fg(COLORS.number)),
                        Cell::from(song.title.as_str()).style(Style::default().fg(COLORS.title)),
                        Cell::from(song.album.as_str()).style(Style::default().fg(COLORS.album)),
                        Cell::from(song.artist.as_str()).style(Style::default().fg(COLORS.artist)),
                    ])
                };

                items.remove(player_index);
                items.insert(player_index, row);

                //Current selection
                if ui_index != player_index {
                    if let Some(song) = songs.get(ui_index) {
                        let row = Row::new(vec![
                            Cell::default(),
                            Cell::from(song.number.to_string())
                                .style(Style::default().bg(COLORS.number)),
                            Cell::from(song.title.as_str())
                                .style(Style::default().bg(COLORS.title)),
                            Cell::from(song.album.as_str())
                                .style(Style::default().bg(COLORS.album)),
                            Cell::from(song.artist.as_str())
                                .style(Style::default().bg(COLORS.artist)),
                        ])
                        .style(Style::default().fg(Color::Black));
                        items.remove(ui_index);
                        items.insert(ui_index, row);
                    }
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

    let t = Table::new(&items)
        .header(
            Row::new(["", "#", "Title", "Album", "Artist"])
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_type(BorderType::Rounded),
        )
        .widths(&con);

    let row_bounds = t.get_row_bounds(ui_index, t.get_row_height(area));

    f.render_stateful_widget(t, area, &mut TableState::new(ui_index));

    Some(row_bounds)
}

fn draw_seeker(player: &mut Player, f: &mut Frame, area: Rect) {
    let block = if player.songs.is_empty() {
        Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
    } else {
        Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
    };

    if log::message().is_some() || player.songs.is_empty() {
        return f.render_widget(block, area);
    }

    let elapsed = player.elapsed().as_secs_f32();
    let duration = player.duration().as_secs_f32();

    let seeker = format!(
        "{:02}:{:02}/{:02}:{:02}",
        (elapsed / 60.0).floor(),
        elapsed.trunc() as u32 % 60,
        (duration / 60.0).floor(),
        duration.trunc() as u32 % 60,
    );

    let ratio = elapsed.floor() / duration;
    let ratio = if ratio.is_nan() {
        0.0
    } else {
        ratio.clamp(0.0, 1.0)
    };

    f.render_widget(
        Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(COLORS.seeker))
            .ratio(ratio as f64)
            .label(seeker),
        area,
    );
}

use crate::widgets::{Cell, Gauge, Row, Table, TableState};
use crossterm::event::KeyModifiers;
use gonk_core::{Colors, Index};
use gonk_player::Player;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use tui::{backend::Backend, Frame};
use unicode_width::UnicodeWidthStr;

const HEADER_HEIGHT: u16 = 5;

pub struct Queue {
    pub ui: Index<()>,
    pub constraint: [u16; 4],
    pub clicked_pos: Option<(u16, u16)>,
    pub player: Player,
    pub colors: Colors,
}

impl Queue {
    pub fn new(vol: u16, colors: Colors) -> Self {
        Self {
            ui: Index::default(),
            constraint: [8, 42, 24, 26],
            clicked_pos: None,
            player: Player::new(vol),
            colors,
        }
    }
    pub fn update(&mut self) {
        if self.ui.is_none() && !self.player.songs.is_empty() {
            self.ui.select(Some(0));
        }
        self.player.update();
    }
    pub fn move_constraint(&mut self, row: usize, modifier: KeyModifiers) {
        if modifier == KeyModifiers::SHIFT && self.constraint[row] != 0 {
            //Move row back.
            self.constraint[row + 1] += 1;
            self.constraint[row] = self.constraint[row].saturating_sub(1);
        } else if self.constraint[row + 1] != 0 {
            //Move row forward.
            self.constraint[row] += 1;
            self.constraint[row + 1] = self.constraint[row + 1].saturating_sub(1);
        }

        debug_assert!(
            self.constraint.iter().sum::<u16>() == 100,
            "Constraint went out of bounds: {:?}",
            self.constraint
        );
    }
    pub fn up(&mut self) {
        self.ui.up_with_len(self.player.songs.len());
    }
    pub fn down(&mut self) {
        self.ui.down_with_len(self.player.songs.len());
    }
    pub fn clear(&mut self) {
        self.player.clear();
        self.ui.select(Some(0));
    }
    pub fn clear_except_playing(&mut self) {
        self.player.clear_except_playing();
        self.ui.select(Some(0));
    }
}

impl Queue {
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.size());

        self.draw_header(f, chunks[0]);

        let row_bounds = self.draw_body(f, chunks[1]);

        //Handle mouse input.
        if let Some((x, y)) = self.clicked_pos {
            let size = f.size();

            //Mouse support for the seek bar.
            if (size.height - 2 == y || size.height - 1 == y) && size.height > 15 {
                let ratio = f64::from(x) / f64::from(size.width);
                let duration = self.player.duration;
                let new_time = duration * ratio;
                self.player.seek_to(new_time);
                self.clicked_pos = None;
            }

            //Mouse support for the queue.
            if let Some((start, _)) = row_bounds {
                //Check if you clicked on the header.
                if y >= HEADER_HEIGHT {
                    let index = (y - HEADER_HEIGHT) as usize + start;

                    //Make sure you didn't click on the seek bar
                    //and that the song index exists.
                    if index < self.player.songs.len()
                        && ((size.height < 15 && y < size.height.saturating_sub(1))
                            || y < size.height.saturating_sub(3))
                    {
                        self.ui.select(Some(index));
                    }
                }
                self.clicked_pos = None;
            }
        }

        self.draw_seeker(f, chunks[2]);
    }
    fn draw_header<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        f.render_widget(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded),
            chunk,
        );

        let state = if self.player.songs.is_empty() {
            String::from("╭─Stopped")
        } else if !self.player.is_paused() {
            String::from("╭─Playing")
        } else {
            String::from("╭─Paused")
        };

        f.render_widget(Paragraph::new(state).alignment(Alignment::Left), chunk);

        if !self.player.songs.is_empty() {
            self.draw_title(f, chunk);
        }

        let volume = Spans::from(format!("Vol: {}%─╮", self.player.volume));
        f.render_widget(Paragraph::new(volume).alignment(Alignment::Right), chunk);
    }
    fn draw_title<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let center = if let Some(song) = self.player.songs.selected() {
            let mut name = song.name.trim_end().to_string();
            let mut album = song.album.trim_end().to_string();
            let mut artist = song.artist.trim_end().to_string();
            let max_width = chunk.width.saturating_sub(30) as usize;

            while artist.width() + name.width() + "-| - |-".width() > max_width {
                if artist.width() > name.width() {
                    artist.pop();
                } else {
                    name.pop();
                }
            }

            while album.width() > max_width {
                album.pop();
            }

            let n = album
                .width()
                .saturating_sub(artist.width() + name.width() + 3);
            let rem = n % 2;
            let pad_front = " ".repeat(n / 2);
            let pad_back = " ".repeat(n / 2 + rem);

            vec![
                Spans::from(vec![
                    Span::raw(format!("─│ {}", pad_front)),
                    Span::styled(artist, Style::default().fg(self.colors.artist)),
                    Span::raw(" ─ "),
                    Span::styled(name, Style::default().fg(self.colors.title)),
                    Span::raw(format!("{} │─", pad_back)),
                ]),
                Spans::from(Span::styled(album, Style::default().fg(self.colors.album))),
            ]
        } else {
            vec![Spans::default(), Spans::default()]
        };

        let center = Paragraph::new(center).alignment(Alignment::Center);
        f.render_widget(center, chunk);
    }
    fn draw_body<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) -> Option<(usize, usize)> {
        if self.player.songs.is_empty() {
            if self.clicked_pos.is_some() {
                self.clicked_pos = None;
            }

            f.render_widget(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::LEFT | Borders::RIGHT),
                chunk,
            );
            return None;
        }

        let (songs, player_index, ui_index) = (
            &self.player.songs.data,
            self.player.songs.selection(),
            self.ui.selection(),
        );

        let mut items: Vec<Row> = songs
            .iter()
            .map(|song| {
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(song.number.to_string())
                        .style(Style::default().fg(self.colors.track)),
                    Cell::from(song.name.as_str()).style(Style::default().fg(self.colors.title)),
                    Cell::from(song.album.as_str()).style(Style::default().fg(self.colors.album)),
                    Cell::from(song.artist.as_str()).style(Style::default().fg(self.colors.artist)),
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
                                .style(Style::default().bg(self.colors.track).fg(Color::Black)),
                            Cell::from(song.name.as_str())
                                .style(Style::default().bg(self.colors.title).fg(Color::Black)),
                            Cell::from(song.album.as_str())
                                .style(Style::default().bg(self.colors.album).fg(Color::Black)),
                            Cell::from(song.artist.as_str())
                                .style(Style::default().bg(self.colors.artist).fg(Color::Black)),
                        ])
                    } else {
                        Row::new(vec![
                            Cell::from(">>").style(
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::DIM | Modifier::BOLD),
                            ),
                            Cell::from(song.number.to_string())
                                .style(Style::default().fg(self.colors.track)),
                            Cell::from(song.name.as_str())
                                .style(Style::default().fg(self.colors.title)),
                            Cell::from(song.album.as_str())
                                .style(Style::default().fg(self.colors.album)),
                            Cell::from(song.artist.as_str())
                                .style(Style::default().fg(self.colors.artist)),
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
                                    .style(Style::default().bg(self.colors.track)),
                                Cell::from(song.name.as_str())
                                    .style(Style::default().bg(self.colors.title)),
                                Cell::from(song.album.as_str())
                                    .style(Style::default().bg(self.colors.album)),
                                Cell::from(song.artist.as_str())
                                    .style(Style::default().bg(self.colors.artist)),
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
            Constraint::Percentage(self.constraint[0]),
            Constraint::Percentage(self.constraint[1]),
            Constraint::Percentage(self.constraint[2]),
            Constraint::Percentage(self.constraint[3]),
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
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded),
            )
            .widths(&con);

        let row_bounds = t.get_row_bounds(ui_index, t.get_row_height(chunk));
        let mut state = TableState::new(ui_index);

        f.render_stateful_widget(t, chunk, &mut state);

        Some(row_bounds)
    }
    fn draw_seeker<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        if self.player.songs.is_empty() {
            return f.render_widget(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT),
                chunk,
            );
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let elapsed = self.player.elapsed();
        let duration = self.player.duration;

        let seeker = format!(
            "{:02}:{:02}/{:02}:{:02}",
            (elapsed / 60.0).floor(),
            elapsed.trunc() as u32 % 60,
            (duration / 60.0).floor(),
            duration.trunc() as u32 % 60,
        );

        let ratio = self.player.elapsed() / self.player.duration;
        let ratio = if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0)
        };

        let g = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(self.colors.seeker))
            .ratio(ratio)
            .label(seeker);

        f.render_widget(g, chunk);
    }
}

use crate::widgets::*;
use crate::*;
use crossterm::event::{KeyModifiers, MouseEvent};
use gonk_player::{Index, Player, Song};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use unicode_width::UnicodeWidthStr;

pub struct Queue {
    pub ui: Index<()>,
    pub constraint: [u16; 4],
    pub player: Player,
}

impl Queue {
    pub fn new(vol: u16) -> Self {
        Self {
            ui: Index::default(),
            constraint: [8, 42, 24, 26],
            player: Player::new(vol),
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
    pub fn delete(&mut self) {
        if let Some(i) = self.ui.index() {
            self.player.delete_song(i);
            //make sure the ui index is in sync
            let len = self.player.songs.len().saturating_sub(1);
            if i > len {
                self.ui.select(Some(len));
            }
        }
    }
    pub fn selected(&self) -> Option<&Song> {
        self.player.songs.selected()
    }
}

impl Queue {
    pub fn draw(&mut self, f: &mut Frame, event: Option<MouseEvent>) {
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

        self.draw_seeker(f, chunks[2]);

        //Don't handle mouse input when the queue is empty.
        if self.player.is_empty() {
            return;
        }

        //Handle mouse input.
        if let Some(event) = event {
            let (x, y) = (event.column, event.row);
            const HEADER_HEIGHT: u16 = 5;

            let size = f.size();

            //Mouse support for the seek bar.
            if (size.height - 2 == y || size.height - 1 == y) && size.height > 15 {
                let ratio = f64::from(x) / f64::from(size.width);
                let duration = self.player.duration;
                let new_time = duration * ratio;
                self.player.seek_to(new_time);
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
            }
        }
    }
    fn draw_header(&mut self, f: &mut Frame, area: Rect) {
        f.render_widget(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded),
            area,
        );

        let state = if self.player.songs.is_empty() {
            String::from("╭─Stopped")
        } else if !self.player.is_paused() {
            String::from("╭─Playing")
        } else {
            String::from("╭─Paused")
        };

        f.render_widget(Paragraph::new(state).alignment(Alignment::Left), area);

        if !self.player.songs.is_empty() {
            self.draw_title(f, area);
        }

        let volume = Spans::from(format!("Vol: {}%─╮", self.player.volume));
        f.render_widget(Paragraph::new(volume).alignment(Alignment::Right), area);
    }
    fn draw_title(&mut self, f: &mut Frame, area: Rect) {
        let title = if let Some(song) = self.player.songs.selected() {
            let mut name = song.name.trim_end().to_string();
            let mut album = song.album.trim_end().to_string();
            let mut artist = song.artist.trim_end().to_string();
            let max_width = area.width.saturating_sub(30) as usize;

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
                    Span::styled(artist, Style::default().fg(COLORS.artist)),
                    Span::raw(" ─ "),
                    Span::styled(name, Style::default().fg(COLORS.name)),
                    Span::raw(format!("{} │─", pad_back)),
                ]),
                Spans::from(Span::styled(album, Style::default().fg(COLORS.album))),
            ]
        } else {
            Vec::new()
        };

        f.render_widget(Paragraph::new(title).alignment(Alignment::Center), area);
    }
    fn draw_body(&mut self, f: &mut Frame, area: Rect) -> Option<(usize, usize)> {
        if self.player.songs.is_empty() {
            f.render_widget(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::LEFT | Borders::RIGHT),
                area,
            );
            return None;
        }

        let (songs, player_index, ui_index) = (
            &self.player.songs.data,
            self.player.songs.index(),
            self.ui.index(),
        );

        let mut items: Vec<Row> = songs
            .iter()
            .map(|song| {
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(song.number.to_string()).style(Style::default().fg(COLORS.number)),
                    Cell::from(song.name.as_str()).style(Style::default().fg(COLORS.name)),
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
                            Cell::from(song.name.as_str())
                                .style(Style::default().bg(COLORS.name).fg(Color::Black)),
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
                            Cell::from(song.name.as_str()).style(Style::default().fg(COLORS.name)),
                            Cell::from(song.album.as_str())
                                .style(Style::default().fg(COLORS.album)),
                            Cell::from(song.artist.as_str())
                                .style(Style::default().fg(COLORS.artist)),
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
                                Cell::from(song.name.as_str())
                                    .style(Style::default().bg(COLORS.name)),
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
            Constraint::Percentage(self.constraint[0]),
            Constraint::Percentage(self.constraint[1]),
            Constraint::Percentage(self.constraint[2]),
            Constraint::Percentage(self.constraint[3]),
        ];

        let t = Table::new(items)
            .header(
                Row::new(["", "Track", "Title", "Album", "Artist"])
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
            // .separator()
            .widths(&con);

        let row_bounds = t.get_row_bounds(ui_index, t.get_row_height(area));

        f.render_stateful_widget(t, area, &mut TableState::new(ui_index));

        Some(row_bounds)
    }
    fn draw_seeker(&mut self, f: &mut Frame, area: Rect) {
        if self.player.songs.is_empty() {
            return f.render_widget(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT),
                area,
            );
        }

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

        f.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .gauge_style(Style::default().fg(COLORS.seeker))
                .ratio(ratio)
                .label(seeker),
            area,
        );
    }
}

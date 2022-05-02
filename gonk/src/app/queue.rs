use crate::widget::{Cell, Gauge, Row, Table, TableState};
use crossterm::event::KeyModifiers;
use gonk_core::{Colors, Index};
use gonk_player::Player;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use tui::{backend::Backend, Frame};
use unicode_width::UnicodeWidthStr;

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
    pub fn move_constraint(&mut self, arg: char, modifier: KeyModifiers) {
        //1 is 48, '1' - 49 = 0
        let i = (arg as usize) - 49;
        if modifier == KeyModifiers::SHIFT && self.constraint[i] != 0 {
            self.constraint[i] = self.constraint[i].saturating_sub(1);
            self.constraint[i + 1] += 1;
        } else if self.constraint[i + 1] != 0 {
            self.constraint[i] += 1;
            self.constraint[i + 1] = self.constraint[i + 1].saturating_sub(1);
        }

        for n in &mut self.constraint {
            if *n > 100 {
                *n = 100;
            }
        }

        assert!(
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
        self.draw_body(f, chunks[1]);
        //TODO: if allow for old and new seeker
        self.draw_new_seeker(f, chunks[2]);
    }
    fn draw_header<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        //Render the borders first
        let b = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded);
        f.render_widget(b, chunk);

        //Left
        let time = if self.player.songs.is_empty() {
            String::from("╭─Stopped")
        } else if !self.player.is_paused() {
            let duration = self.player.duration;
            let elapsed = self.player.elapsed();

            let mins = elapsed / 60.0;
            let rem = elapsed % 60.0;
            let e = format!("{:02}:{:02}", mins.trunc(), rem.trunc());

            let mins = duration / 60.0;
            let rem = duration % 60.0;
            let d = format!("{:02}:{:02}", mins.trunc(), rem.trunc());

            format!("╭─{}/{}", e, d)
        } else {
            String::from("╭─Paused")
        };

        let left = Paragraph::new(time).alignment(Alignment::Left);
        f.render_widget(left, chunk);

        //Center
        if !self.player.songs.is_empty() {
            self.draw_title(f, chunk);
        }

        //Right
        let text = Spans::from(format!("Vol: {}%─╮", self.player.volume));
        let right = Paragraph::new(text).alignment(Alignment::Right);
        f.render_widget(right, chunk);
    }
    fn draw_title<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let center = if let Some(song) = self.player.songs.selected() {
            let mut name = song.name.trim_end().to_string();
            let mut album = song.album.trim_end().to_string();
            let mut artist = song.artist.trim_end().to_string();
            let width = chunk.width.saturating_sub(30) as usize;

            while artist.width() + name.width() + "-| - |-".width() > width {
                if artist.width() > name.width() {
                    artist.pop();
                } else {
                    name.pop();
                }
            }

            while album.width() > width {
                album.pop();
            }

            let title = format!("{} - {}", artist, name);

            //The code did this:
            // |          Artist - Track Name          |
            // | A very very very very long album name |

            // let (album_front, album_back) = if album.width() > title.width() {
            //     ("│ ", " │")
            // } else {
            //     ("", "")
            // };

            let mut pad_front = String::new();
            let mut pad_back = String::new();
            let mut i = 0;

            while title.width() + pad_front.width() + pad_back.width() < album.width() {
                if i % 2 == 0 {
                    pad_front.push(' ');
                } else {
                    pad_back.push(' ');
                }
                i += 1;
            }

            vec![
                Spans::from(vec![
                    Span::raw(format!("─│ {}", pad_front)),
                    Span::styled(artist, Style::default().fg(self.colors.artist)),
                    Span::raw(" - "),
                    Span::styled(name, Style::default().fg(self.colors.title)),
                    Span::raw(format!("{} │─", pad_back)),
                ]),
                Spans::from(vec![
                    // Span::raw(album_front),
                    Span::styled(album, Style::default().fg(self.colors.album)),
                    // Span::raw(album_back),
                ]),
            ]
        } else {
            vec![Spans::default(), Spans::default()]
        };

        let center = Paragraph::new(center).alignment(Alignment::Center);
        f.render_widget(center, chunk);
    }
    fn draw_body<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        if self.player.songs.is_empty() {
            self.handle_mouse(f.size(), &Table::default(), None, chunk);
            return f.render_widget(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::LEFT | Borders::RIGHT),
                chunk,
            );
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
                    Cell::from(song.name.clone()).style(Style::default().fg(self.colors.title)),
                    Cell::from(song.album.clone()).style(Style::default().fg(self.colors.album)),
                    Cell::from(song.artist.clone()).style(Style::default().fg(self.colors.artist)),
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
                            Cell::from(song.name.clone())
                                .style(Style::default().bg(self.colors.title).fg(Color::Black)),
                            Cell::from(song.album.clone())
                                .style(Style::default().bg(self.colors.album).fg(Color::Black)),
                            Cell::from(song.artist.clone())
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
                            Cell::from(song.name.clone())
                                .style(Style::default().fg(self.colors.title)),
                            Cell::from(song.album.clone())
                                .style(Style::default().fg(self.colors.album)),
                            Cell::from(song.artist.clone())
                                .style(Style::default().fg(self.colors.artist)),
                        ])
                    };

                    items.remove(player_index);
                    items.insert(player_index, row);

                    //Current selection
                    if ui_index != player_index {
                        if let Some(song) = songs.get(ui_index) {
                            let row = Row::new(vec![
                                Cell::from(""),
                                Cell::from(song.number.to_string())
                                    .style(Style::default().bg(self.colors.track)),
                                Cell::from(song.name.clone())
                                    .style(Style::default().bg(self.colors.title)),
                                Cell::from(song.album.clone())
                                    .style(Style::default().bg(self.colors.album)),
                                Cell::from(song.artist.clone())
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

        self.handle_mouse(f.size(), &t, ui_index, chunk);

        //required to scroll songs
        let mut state = TableState::new(ui_index);
        f.render_stateful_widget(t, chunk, &mut state);
    }
    fn seeker(&mut self, size: Rect) {
        if let Some((width, height)) = self.clicked_pos {
            if size.height - 3 == height || size.height - 2 == height || size.height - 1 == height {
                let ratio = f64::from(width) / f64::from(size.width);
                let duration = self.player.duration;
                let new_time = duration * ratio;
                self.player.seek_to(new_time);
                self.clicked_pos = None;
            }
        }
    }
    //TODO: handle very small window sizes
    fn handle_mouse(&mut self, size: Rect, table: &Table, ui_index: Option<usize>, chunk: Rect) {
        self.seeker(size);

        if let Some((_, height)) = self.clicked_pos {
            let (start, _) = table.get_row_bounds(ui_index, table.get_row_height(chunk));
            //the header is 5 units long
            if height >= 5 {
                let index = (height - 5) as usize + start;
                //don't select out of bounds
                if index < self.player.songs.len() && height < size.height.saturating_sub(4) {
                    self.ui.select(Some(index));
                }
            }
            self.clicked_pos = None;
        }
    }
    fn draw_new_seeker<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
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

        self.seeker(f.size());
    }
}

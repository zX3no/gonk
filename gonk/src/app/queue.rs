use crate::index::Index;
use crossterm::event::KeyModifiers;
use gonk_types::Song;
use rand::{prelude::SliceRandom, thread_rng};
use rodio::Player;
use std::time::Duration;
use tui::{backend::Backend, Frame};

pub struct Queue {
    pub ui: Index<()>,
    pub list: Index<Song>,
    pub constraint: [u16; 4],
    pub clicked_pos: Option<(u16, u16)>,
    pub player: Player,
}

impl Queue {
    pub fn new(volume: u16) -> Self {
        Self {
            ui: Index::default(),
            list: Index::default(),
            constraint: [8, 42, 24, 26],
            clicked_pos: None,
            player: Player::new(volume),
        }
    }
    pub fn volume_up(&mut self) {
        self.player.change_volume(true);
    }
    pub fn volume_down(&mut self) {
        self.player.change_volume(false);
    }
    pub fn play(&self) {
        if self.player.is_paused() {
            self.player.toggle_playback();
        }
    }
    pub fn play_pause(&self) {
        self.player.toggle_playback();
    }
    pub fn update(&mut self) {
        if self.player.trigger_next() {
            self.next();
        }
    }
    pub fn prev(&mut self) {
        self.list.up();
        self.play_selected();
    }
    pub fn next(&mut self) {
        self.list.down();
        self.play_selected();
    }
    pub fn clear(&mut self) {
        self.list = Index::default();
        self.player.stop();
    }
    pub fn up(&mut self) {
        self.ui.up_with_len(self.list.len());
    }
    pub fn down(&mut self) {
        self.ui.down_with_len(self.list.len());
    }
    pub fn add(&mut self, mut songs: Vec<Song>) {
        if self.list.is_empty() {
            self.list.append(&mut songs);
            self.list.select(Some(0));
            self.ui.select(Some(0));
            self.play_selected();
        } else {
            self.list.append(&mut songs);
        }
    }
    pub fn select(&mut self) {
        if let Some(index) = self.ui.index {
            self.list.select(Some(index));
            self.play_selected();
        }
    }
    pub fn delete_selected(&mut self) {
        if let Some(index) = self.ui.index {
            //remove the item from the ui
            self.list.remove(index);
            if let Some(playing) = self.list.index {
                let len = self.list.len();

                if len == 0 {
                    self.clear();
                    return;
                } else if playing == index && index == 0 {
                    self.list.select(Some(0));
                } else if playing == index && len == index {
                    self.list.select(Some(len - 1));
                } else if index < playing {
                    self.list.select(Some(playing - 1));
                }

                let end = self.list.len().saturating_sub(1);
                if index > end {
                    self.ui.select(Some(end));
                }
                //if the playing song was deleted
                //play the next track
                if index == playing {
                    self.play_selected();
                }
            };
        }
    }
    pub fn play_selected(&mut self) {
        if let Some(item) = self.list.selected() {
            self.player.play(&item.path);
        } else {
            self.player.stop();
        }
    }
    pub fn seek_fw(&mut self) {
        self.play();
        self.player.seek_fw();
    }
    pub fn seek_bw(&mut self) {
        self.play();
        self.player.seek_bw();
    }
    pub fn duration(&self) -> Option<f64> {
        if self.list.is_empty() {
            None
        } else {
            self.player.duration()
        }
    }
    pub fn seek_to(&self, new_time: f64) {
        self.player.seek_to(Duration::from_secs_f64(new_time));
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

        if self.constraint.iter().sum::<u16>() != 100 {
            panic!("Constraint went out of bounds: {:?}", self.constraint);
        }
    }
    pub fn randomize(&mut self) {
        if let Some(song) = self.list.selected().cloned() {
            self.list.data.shuffle(&mut thread_rng());

            let mut index = 0;
            for (i, s) in self.list.data.iter().enumerate() {
                if s == &song {
                    index = i;
                }
            }
            self.list.select(Some(index));
        }
    }
    pub(crate) fn change_output_device(&mut self, device: &rodio::Device) {
        let pos = self.player.elapsed();
        self.player.change_output_device(device);
        self.play_selected();
        //TODO: when audio does not play, this gets reset to 0
        self.seek_to(pos.as_secs_f64());
    }
}

use gonk_database::Colors;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState};

impl Queue {
    fn handle_mouse<B: Backend>(&mut self, f: &mut Frame<B>) {
        //Songs
        if let Some((_, row)) = self.clicked_pos {
            let size = f.size();
            let height = size.height as usize;
            let len = self.list.len();
            if height > 7 {
                if height - 7 < len {
                    //TODO: I have no idea how to figure out what index i clicked on
                } else {
                    let start_row = 5;
                    if row >= start_row {
                        let index = (row - start_row) as usize;
                        if index < len {
                            self.ui.select(Some(index));
                        }
                    }
                }
            }
        }

        //Seeker
        if let Some((column, row)) = self.clicked_pos {
            let size = f.size();
            if size.height - 3 == row
                || size.height - 2 == row
                || size.height - 1 == row && column >= 3 && column < size.width - 2
            {
                let ratio = (column - 3) as f64 / size.width as f64;
                if let Some(duration) = self.duration() {
                    let new_time = duration * ratio;
                    self.seek_to(new_time);
                    self.play();
                }
            }
            self.clicked_pos = None;
        }
    }
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, colors: &Colors) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(f.size());

        self.handle_mouse(f);
        self.draw_header(f, chunks[0], colors);
        self.draw_songs(f, chunks[1], colors);
        self.draw_seeker(f, chunks[2]);
    }
    fn draw_header<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect, colors: &Colors) {
        //Render the borders first
        let b = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded);
        f.render_widget(b, chunk);

        //Left
        let time = if self.list.is_empty() {
            String::from("╭─Stopped")
        } else if !self.player.is_paused() {
            if let Some(duration) = self.duration() {
                let elapsed = self.player.elapsed().as_secs_f64();

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

        //Center
        let center = if let Some(song) = self.list.selected() {
            //I wish that paragraphs had clipping
            //I think constraints do
            //I could render the -| |- on a seperate layer
            //outside of the constraint which might work better?

            //clip the name so it doesn't overflow
            let mut name = song.name.clone();
            const MAX_WIDTH: u16 = 60;
            while (name.len() + song.artist.len() + "─| - |─".len())
                > (chunk.width - MAX_WIDTH) as usize
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

        //TODO: scroll the text to the left
        let center = Paragraph::new(center).alignment(Alignment::Center);
        f.render_widget(center, chunk);

        //Right
        let volume = self.player.volume();
        let text = Spans::from(format!("Vol: {}%─╮", volume));
        let right = Paragraph::new(text).alignment(Alignment::Right);
        f.render_widget(right, chunk);
    }
    fn draw_songs<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect, colors: &Colors) {
        if self.list.is_empty() {
            return f.render_widget(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded),
                chunk,
            );
        }

        let (songs, now_playing, ui_index) = (&self.list.data, self.list.index, self.ui.index);

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
                            Cell::from(song.name.to_owned())
                                .style(Style::default().fg(colors.title)),
                            Cell::from(song.album.to_owned())
                                .style(Style::default().fg(colors.album)),
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
                            Cell::from(song.name.to_owned())
                                .style(Style::default().bg(colors.title)),
                            Cell::from(song.album.to_owned())
                                .style(Style::default().bg(colors.album)),
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
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded),
            )
            .widths(&con);

        //this is so that scrolling works
        let mut state = TableState::default();
        state.select(ui_index);

        f.render_stateful_widget(t, chunk, &mut state);
    }
    fn draw_seeker<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        if self.list.is_empty() {
            return f.render_widget(
                Block::default()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded),
                chunk,
            );
        }

        let area = f.size();
        let width = area.width;
        let percent = self.player.seeker();
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
}

use super::TOML;
use crate::widget::{Cell, Row, Table, TableState};
use crossterm::event::KeyModifiers;
use gonk_tcp::Client;
use gonk_types::{Index, Song};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph};
use tui::{backend::Backend, Frame};

#[derive(Default)]
pub struct ScrollText {
    string: String,
    scroll_string: String,
    first: usize,
    max: usize,
}

#[allow(unused)]
impl ScrollText {
    pub fn new(string: &str, max: usize) -> Self {
        let mut s = Self {
            string: string.to_string(),
            scroll_string: String::new(),
            first: 0,
            max,
        };
        s.next();
        s
    }
    pub fn current(&self) -> String {
        self.scroll_string.clone()
    }
    pub fn next(&mut self) {
        let string = self.string.clone();

        let last = self.first + self.max;
        let mut new_string = String::new();

        for i in self.first..last {
            if i >= string.len() {
                if let Some(char) = string.chars().nth(i - string.len()) {
                    new_string.push(char);
                }
            } else {
                new_string.push(string.chars().nth(i).unwrap());
            }
        }

        if string.chars().nth(self.first).is_none() {
            self.first = 0;
        }

        self.first += 1;

        self.scroll_string = new_string;
    }
    pub fn is_empty(&self) -> bool {
        self.string.is_empty()
    }
}

#[derive(Default)]
pub struct ServerState {
    //TODO: replace with index
    pub queue: Vec<Song>,
    pub selected: Option<Song>,
    pub index: Option<usize>,

    //TODO: change to queue.len()
    pub total_songs: usize,

    pub elapsed: f64,

    //TODO: should duration be removed from gonk_types::Song
    pub duration: f64,

    //TODO: why not precalculate the percentage on the server side?
    pub seeker: f64,

    //TODO: change to queue.is_empty()
    pub empty: bool,

    pub paused: bool,

    pub volume: u16,
}

pub struct Queue {
    pub ui: Index<()>,
    pub constraint: [u16; 4],
    pub clicked_pos: Option<(u16, u16)>,
    pub scroll_text: ScrollText,
    // pub player: Player,
    pub client: Client,
    pub server_state: ServerState,
}

impl Queue {
    pub fn new(start_vol: u16) -> Self {
        optick::event!("new queue");
        Self {
            ui: Index::default(),
            constraint: [8, 42, 24, 26],
            clicked_pos: None,
            scroll_text: ScrollText::default(),
            // player: Player::new(start_vol),
            client: Client::new(),
            server_state: ServerState::default(),
        }
    }
    pub fn update(&mut self) {
        if self.ui.is_none() && !self.server_state.empty {
            self.ui.select(Some(0));
        }
        self.scroll_text.next();
    }
    #[allow(unused)]
    fn update_text(&mut self) {
        if let Some(song) = &self.server_state.selected {
            let mut name = format!("{} - {}", &song.artist, &song.name);

            //TODO: this is broken
            //pad the string
            for _ in 0..3 {
                name.push(' ');
                name.insert(0, ' ');
            }

            self.scroll_text = ScrollText::new(&name, 25);
        } else {
            self.scroll_text = ScrollText::default();
        }
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
        self.ui.up_with_len(self.server_state.total_songs);
    }
    pub fn down(&mut self) {
        self.ui.down_with_len(self.server_state.total_songs)
    }

    pub fn clear(&mut self) {
        self.client.clear_songs();
        self.ui = Index::default();
    }
}

impl Queue {
    fn handle_mouse<B: Backend>(&mut self, f: &mut Frame<B>) {
        //Songs
        if let Some((_, row)) = self.clicked_pos {
            let size = f.size();
            let height = size.height as usize;
            let len = self.server_state.total_songs;
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
                let ratio = f64::from(column - 3) / f64::from(size.width);
                let duration = self.server_state.duration;
                let new_time = duration * ratio;
                self.client.seek_to(new_time);
            }
            self.clicked_pos = None;
        }
    }
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        optick::event!("draw Queue");
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(f.size());

        self.handle_mouse(f);
        self.draw_header(f, chunks[0]);
        self.draw_songs(f, chunks[1]);
        self.draw_seeker(f, chunks[2]);
    }
    fn draw_header<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        //Render the borders first
        let b = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded);
        f.render_widget(b, chunk);

        //Left
        let time = if self.server_state.empty {
            String::from("╭─Stopped")
        } else if !self.server_state.paused {
            let duration = self.server_state.duration;
            let elapsed = self.server_state.elapsed;

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
        if !self.server_state.empty {
            self.draw_scrolling_text_old(f, chunk);
        }

        //Right
        let text = Spans::from(format!("Vol: {}%─╮", self.server_state.volume));
        let right = Paragraph::new(text).alignment(Alignment::Right);
        f.render_widget(right, chunk);
    }
    fn draw_scrolling_text_old<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        let center = if let Some(song) = &self.server_state.selected {
            //I wish that paragraphs had clipping
            //I think constraints do
            //I could render the -| |- on a seperate layer
            //outside of the constraint which might work better?

            //clip the name so it doesn't overflow
            const MAX_WIDTH: u16 = 60;
            let mut artist = song.artist.clone();

            //TODO: this while loop is dangerous
            //might want a few safe guards or swap to for loop.
            while (artist.len() + song.name.len() + "─| - |─".len())
                > (chunk.width - MAX_WIDTH) as usize
            {
                if artist.is_empty() {
                    break;
                }
                artist.pop();
            }

            vec![
                Spans::from(vec![
                    Span::raw("─| "),
                    Span::styled(
                        artist.trim_end().to_string(),
                        Style::default().fg(TOML.colors.artist),
                    ),
                    Span::raw(" - "),
                    Span::styled(&song.name, Style::default().fg(TOML.colors.title)),
                    Span::raw(" |─"),
                ]),
                Spans::from(Span::styled(
                    &song.album,
                    Style::default().fg(TOML.colors.album),
                )),
            ]
        } else {
            vec![Spans::default(), Spans::default()]
        };

        //TODO: scroll the text to the left
        let center = Paragraph::new(center).alignment(Alignment::Center);
        f.render_widget(center, chunk);
    }
    #[allow(unused)]
    fn draw_scrolling_text<B: Backend>(&mut self, f: &mut Frame<B>, chunk: Rect) {
        if self.scroll_text.is_empty() {
            self.update_text();
        }

        //TODO: The text is not centered
        let p = Paragraph::new(Spans::from(format!("─| {} |─", self.scroll_text.current())))
            .alignment(Alignment::Center);

        f.render_widget(p, chunk);
    }
    fn draw_songs<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        if self.server_state.empty {
            return f.render_widget(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded),
                chunk,
            );
        }

        let (songs, now_playing, ui_index) = (
            &self.server_state.queue,
            self.server_state.index,
            self.ui.index,
        );

        let mut items: Vec<Row> = songs
            .iter()
            .map(|song| {
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(song.number.to_string())
                        .style(Style::default().fg(TOML.colors.track)),
                    Cell::from(song.name.clone()).style(Style::default().fg(TOML.colors.title)),
                    Cell::from(song.album.clone()).style(Style::default().fg(TOML.colors.album)),
                    Cell::from(song.artist.clone()).style(Style::default().fg(TOML.colors.artist)),
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
                                .style(Style::default().bg(TOML.colors.track).fg(Color::Black)),
                            Cell::from(song.name.clone())
                                .style(Style::default().bg(TOML.colors.title).fg(Color::Black)),
                            Cell::from(song.album.clone())
                                .style(Style::default().bg(TOML.colors.album).fg(Color::Black)),
                            Cell::from(song.artist.clone())
                                .style(Style::default().bg(TOML.colors.artist).fg(Color::Black)),
                        ])
                    } else {
                        Row::new(vec![
                            Cell::from(">>").style(
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::DIM | Modifier::BOLD),
                            ),
                            Cell::from(song.number.to_string())
                                .style(Style::default().fg(TOML.colors.track)),
                            Cell::from(song.name.clone())
                                .style(Style::default().fg(TOML.colors.title)),
                            Cell::from(song.album.clone())
                                .style(Style::default().fg(TOML.colors.album)),
                            Cell::from(song.artist.clone())
                                .style(Style::default().fg(TOML.colors.artist)),
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
                                .style(Style::default().bg(TOML.colors.track)),
                            Cell::from(song.name.clone())
                                .style(Style::default().bg(TOML.colors.title)),
                            Cell::from(song.album.clone())
                                .style(Style::default().bg(TOML.colors.album)),
                            Cell::from(song.artist.clone())
                                .style(Style::default().bg(TOML.colors.artist)),
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

        //required to scroll songs
        let mut state = TableState::new(ui_index);
        f.render_stateful_widget(t, chunk, &mut state);
    }
    fn draw_seeker<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        if self.server_state.empty {
            return f.render_widget(
                Block::default()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded),
                chunk,
            );
        }

        let area = f.size();
        let width = area.width;
        let percent = self.server_state.seeker;
        //TOOD: casting to usize could fail
        let pos = (f64::from(width) * percent) as usize;

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

        f.render_widget(p, chunk);
    }
}

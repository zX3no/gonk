use super::queue::Queue;
use gonk::Frame;
use gonk_core::{sqlite, Colors};
use std::time::{Duration, Instant};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

const WAIT_TIME: Duration = Duration::from_secs(2);

pub struct StatusBar {
    dots: usize,
    colors: Colors,
    busy: bool,
    scan_message: String,
    wait_timer: Option<Instant>,
    scan_timer: Option<Instant>,
    hidden: bool,
}

impl StatusBar {
    pub fn new(colors: Colors) -> Self {
        Self {
            dots: 1,
            colors,
            busy: false,
            scan_message: String::new(),
            wait_timer: None,
            scan_timer: None,
            hidden: true,
        }
    }

    //Updates the dots in "Scanning for files .."
    pub fn update(&mut self, db_busy: bool, queue: &Queue) {
        if db_busy {
            if self.dots < 3 {
                self.dots += 1;
            } else {
                self.dots = 1;
            }
        } else {
            self.dots = 1;
        }

        if let Some(timer) = self.wait_timer {
            if timer.elapsed() >= WAIT_TIME {
                self.wait_timer = None;
                self.busy = false;

                //FIXME: If the queue was not empty
                //and the status bar was hidden
                //before triggering an update
                //the status bar will stay open
                //without the users permission.
                if queue.player.is_empty() {
                    self.hidden = true;
                }
            }
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.hidden = !self.hidden;
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}
impl StatusBar {
    pub fn draw(&mut self, area: Rect, f: &mut Frame, busy: bool, queue: &Queue) {
        if !self.busy {
            if busy {
                self.hidden = false;
                self.busy = true;
                self.scan_timer = Some(Instant::now());
            }
        } else {
            //Stop scan time and start wait timer.
            if let Some(scan_time) = self.scan_timer {
                self.busy = false;
                self.wait_timer = Some(Instant::now());
                self.scan_timer = None;
                self.scan_message = format!(
                    "Finished adding {} files in {:.2} seconds.",
                    sqlite::total_songs(),
                    scan_time.elapsed().as_secs_f32(),
                );
            }
        }

        if self.hidden {
            return;
        }

        let text = if busy {
            Spans::from(format!("Scannig for files{}", ".".repeat(self.dots)))
        } else if self.wait_timer.is_some() {
            Spans::from(self.scan_message.as_str())
        } else {
            if let Some(song) = queue.selected() {
                Spans::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        song.number.to_string(),
                        Style::default().fg(self.colors.number),
                    ),
                    Span::raw(" ｜ "),
                    Span::styled(song.name.as_str(), Style::default().fg(self.colors.name)),
                    Span::raw(" ｜ "),
                    Span::styled(song.album.as_str(), Style::default().fg(self.colors.album)),
                    Span::raw(" ｜ "),
                    Span::styled(
                        song.artist.as_str(),
                        Style::default().fg(self.colors.artist),
                    ),
                ])
            } else {
                Spans::default()
            }
        };

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(85), Constraint::Percentage(15)])
            .split(area);

        f.render_widget(
            Paragraph::new(text).alignment(Alignment::Left).block(
                Block::default()
                    .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded),
            ),
            area[0],
        );

        //TODO: Draw mini progress bar here.

        let text = if queue.player.is_paused() {
            String::from("Paused ")
        } else {
            format!("Vol: {}% ", queue.player.volume)
        };

        f.render_widget(
            Paragraph::new(text).alignment(Alignment::Right).block(
                Block::default()
                    .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
                    .border_type(BorderType::Rounded),
            ),
            area[1],
        );
    }
}

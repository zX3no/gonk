use std::time::{Duration, Instant};

use super::queue::Queue;
use gonk::Frame;
use gonk_core::{sqlite, Colors};
use tui::{
    layout::Rect,
    style::Style,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

const MESSAGE_TIME: Duration = Duration::from_secs(2);

//TODO:
//This bar should show in all pages but the queue.
//It will show what the current song is, how much is left and the volume.
//It should also show when the database is updating.
pub struct StatusBar {
    dots: usize,
    colors: Colors,
    busy: bool,
    scan_message: String,
    wait_timer: Option<Instant>,
    scan_timer: Option<Instant>,
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
        }
    }

    //Updates the dots in "Scanning for files .."
    pub fn update(&mut self, db_busy: bool) {
        if db_busy {
            if self.dots < 4 {
                self.dots += 1;
            } else {
                self.dots = 1;
            }
        } else {
            self.dots = 1;
        }

        if let Some(timer) = self.wait_timer {
            if timer.elapsed() >= MESSAGE_TIME {
                self.wait_timer = None;
                self.busy = false;
            }
        }
    }

    //TODO: Add a keybind to hide the status_bar.
    //Should also be an option in the config file.
    pub fn is_hidden(&self) -> bool {
        false
    }

    pub fn is_busy(&self) -> bool {
        self.wait_timer.is_some() || self.busy
    }
}
impl StatusBar {
    pub fn draw(&mut self, area: Rect, f: &mut Frame, busy: bool, queue: &Queue) {
        if self.is_hidden() {
            return;
        }

        if busy {
            self.busy = true;
            self.scan_timer = Some(Instant::now());
        } else if self.busy {
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

        let text = if busy {
            Spans::from(format!("Scannig for files{}", ".".repeat(self.dots)))
        } else if self.wait_timer.is_some() {
            Spans::from(self.scan_message.as_str())
        } else {
            if let Some(song) = queue.selected() {
                Spans::from(vec![
                    Span::styled(
                        song.number.to_string(),
                        Style::default().fg(self.colors.number),
                    ),
                    Span::raw(" - "),
                    Span::styled(song.name.as_str(), Style::default().fg(self.colors.name)),
                    Span::raw(" - "),
                    Span::styled(song.album.as_str(), Style::default().fg(self.colors.album)),
                    Span::raw(" - "),
                    Span::styled(
                        song.artist.as_str(),
                        Style::default().fg(self.colors.artist),
                    ),
                ])
            } else {
                return;
            }
        };

        f.render_widget(
            Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            area,
        );
    }
}

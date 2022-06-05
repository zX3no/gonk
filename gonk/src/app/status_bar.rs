use gonk::Frame;
use gonk_core::Colors;
use tui::{
    layout::Rect,
    style::Style,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::queue::Queue;

//TODO:
//This bar should show in all pages but the queue.
//It will show what the current song is, how much is left and the volume.
//It should also show when the database is updating.
pub struct StatusBar {
    dots: usize,
    colors: Colors,
}

impl StatusBar {
    pub fn new(colors: Colors) -> Self {
        Self { dots: 1, colors }
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
    }

    //TODO: Add a keybind to hide the status_bar.
    //Should also be an option in the config file.
    pub fn is_hidden(&self) -> bool {
        false
    }
}
impl StatusBar {
    pub fn draw(&mut self, area: Rect, f: &mut Frame, busy: bool, queue: &Queue) {
        if self.is_hidden() {
            return;
        }

        let text = if busy {
            Spans::from(format!("Scannig for files{}", ".".repeat(self.dots)))
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

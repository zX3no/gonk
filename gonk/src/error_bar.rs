use crate::{Frame, ERROR, SHOW_ERROR};
use std::time::{Duration, Instant};
use tui::{
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Borders, Paragraph},
};

const WAIT_TIME: Duration = Duration::from_secs(2);

pub struct ErrorBar {
    pub timer: Option<Instant>,
}

impl ErrorBar {
    pub fn new() -> Self {
        Self { timer: None }
    }
    pub fn start(&mut self) {
        self.timer = Some(Instant::now());
    }
}

pub fn draw(error_bar: &mut ErrorBar, area: Rect, f: &mut Frame) {
    if let Some(timer) = error_bar.timer {
        if timer.elapsed() >= WAIT_TIME {
            error_bar.timer = None;
            unsafe {
                SHOW_ERROR = false;
                ERROR = String::new();
            }
        }
    }

    let message = unsafe { ERROR.as_str() };

    f.render_widget(
        Paragraph::new(message).alignment(Alignment::Left).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        area,
    );
}

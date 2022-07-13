use crate::Frame;
use std::{
    thread,
    time::{Duration, Instant},
};
use tui::{
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Borders, Paragraph},
};

const WAIT_TIME: Duration = Duration::from_secs(2);

pub static mut LOG: Log = Log {
    message: String::new(),
    timer: None,
};

pub struct Log {
    pub message: String,
    pub timer: Option<Instant>,
}

pub fn init() {
    thread::spawn(|| loop {
        unsafe {
            if let Some(timer) = LOG.timer {
                if timer.elapsed() >= WAIT_TIME {
                    LOG.timer = None;
                    LOG.message = String::new();
                }
            }
        }
    });
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::time::Instant;
        use crate::log::LOG;

        unsafe {
           LOG.message = format_args!($($arg)*).to_string();
           LOG.timer = Some(Instant::now());
        }
    }
    };
}

pub fn message() -> Option<&'static str> {
    unsafe {
        if LOG.message.is_empty() {
            None
        } else {
            Some(LOG.message.as_str())
        }
    }
}

pub fn draw(area: Rect, f: &mut Frame) {
    let message = message().unwrap_or_else(|| "");

    f.render_widget(
        Paragraph::new(message).alignment(Alignment::Left).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        area,
    );
}

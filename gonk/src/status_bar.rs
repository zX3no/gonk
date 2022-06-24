use crate::{Frame, COLORS};
use gonk_database::query;
use gonk_player::Player;
use std::time::{Duration, Instant};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph},
};

const WAIT_TIME: Duration = Duration::from_secs(2);

pub struct StatusBar {
    pub dots: usize,
    pub busy: bool,
    pub scan_message: String,
    pub wait_timer: Option<Instant>,
    pub scan_timer: Option<Instant>,
    pub hidden: bool,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            dots: 1,
            busy: false,
            scan_message: String::new(),
            wait_timer: None,
            scan_timer: None,
            hidden: true,
        }
    }
}

//Updates the dots in "Scanning for files .."
pub fn update(status_bar: &mut StatusBar, db_busy: bool, player: &Player) {
    if db_busy {
        if status_bar.dots < 3 {
            status_bar.dots += 1;
        } else {
            status_bar.dots = 1;
        }
    } else {
        status_bar.dots = 1;
    }

    if let Some(timer) = status_bar.wait_timer {
        if timer.elapsed() >= WAIT_TIME {
            status_bar.wait_timer = None;
            status_bar.busy = false;

            //FIXME: If the queue was not empty
            //and the status bar was hidden
            //before triggering an update
            //the status bar will stay open
            //without the users permission.
            if player.is_empty() {
                status_bar.hidden = true;
            }
        }
    }
}

pub fn draw(status_bar: &mut StatusBar, area: Rect, f: &mut Frame, busy: bool, player: &Player) {
    if busy {
        //If database is busy but status_bar is not
        //set the status bar to busy
        if !status_bar.busy {
            status_bar.busy = true;
            status_bar.hidden = false;
            status_bar.scan_timer = Some(Instant::now());
        }
    } else if status_bar.busy {
        //If database is no-longer busy
        //but status bar is. Print the duration
        //and start the wait timer.
        if let Some(scan_time) = status_bar.scan_timer {
            status_bar.busy = false;
            status_bar.wait_timer = Some(Instant::now());
            status_bar.scan_timer = None;
            status_bar.scan_message = format!(
                "Finished adding {} files in {:.2} seconds.",
                query::total_songs(),
                scan_time.elapsed().as_secs_f32(),
            );
        }
    }

    if status_bar.hidden {
        return;
    }

    let text = if busy {
        Spans::from(format!("Scannig for files{}", ".".repeat(status_bar.dots)))
    } else if status_bar.wait_timer.is_some() {
        Spans::from(status_bar.scan_message.as_str())
    } else if let Some(song) = player.songs.selected() {
        Spans::from(vec![
            Span::raw(" "),
            Span::styled(song.number.to_string(), Style::default().fg(COLORS.number)),
            Span::raw(" ｜ "),
            Span::styled(song.name.as_str(), Style::default().fg(COLORS.name)),
            Span::raw(" ｜ "),
            Span::styled(song.album.as_str(), Style::default().fg(COLORS.album)),
            Span::raw(" ｜ "),
            Span::styled(song.artist.as_str(), Style::default().fg(COLORS.artist)),
        ])
    } else {
        Spans::default()
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

    let text = if player.is_paused() {
        String::from("Paused ")
    } else {
        format!("Vol: {}% ", player.volume)
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

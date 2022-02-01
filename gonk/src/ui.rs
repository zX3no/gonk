use crate::app::{App, AppMode};
use tui::{backend::Backend, style::Color, Frame};

mod browser;
mod options;
mod queue;
mod search;

static TRACK: Color = Color::Green;
static TITLE: Color = Color::Cyan;
static ALBUM: Color = Color::Magenta;
static ARTIST: Color = Color::Blue;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    match app.app_mode {
        AppMode::Browser => browser::draw(f, &app.browser),
        AppMode::Queue => queue::draw(f, app),
        AppMode::Search => search::draw(f, &app.search, app.db),
        AppMode::Options => options::draw(f, &app.options, app.db),
    }
}

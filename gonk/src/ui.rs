use crate::app::{App, AppMode};
use gonk_database::Toml;
use tui::{backend::Backend, style::Color, Frame};

mod browser;
mod options;
mod queue;
mod search;

pub struct Colors {
    track: Color,
    title: Color,
    album: Color,
    artist: Color,
}

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    //TODO: handle failed serialization
    let toml = Toml::new().unwrap().colors;
    let colors = Colors {
        track: toml.track,
        title: toml.title,
        album: toml.album,
        artist: toml.artist,
    };

    match app.app_mode {
        AppMode::Browser => browser::draw(f, &app.browser),
        AppMode::Queue => queue::draw(f, app, colors),
        AppMode::Search => search::draw(f, &app.search, app.db, colors),
        AppMode::Options => options::draw(f, &app.options, app.db),
    }
}

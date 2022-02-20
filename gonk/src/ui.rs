use crate::app::{App, AppMode};
use tui::{backend::Backend, Frame};

mod browser;
mod options;
mod queue;
mod search;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    //TODO: handle failed serialization
    let colors = app.options.colors();

    match app.app_mode {
        AppMode::Browser => browser::draw(f, &app.browser),
        AppMode::Queue => queue::draw(f, &app.queue, colors),
        AppMode::Search => search::draw(f, &app.search, app.db, colors),
        AppMode::Options => options::draw(f, &app.options, app.db),
    }

    handle_mouse(f, app);
}

pub fn handle_mouse<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    if let AppMode::Queue = app.app_mode {
        let queue = &mut app.queue;
        //Songs
        if let Some((_, row)) = queue.clicked_pos {
            let size = f.size();
            let height = size.height as usize;
            let len = queue.list.len();
            if height > 7 {
                if height - 7 < len {
                    //TODO: I have no idea how to figure out what index i clicked on
                } else {
                    let start_row = 5;
                    if row >= start_row {
                        let index = (row - start_row) as usize;
                        if index < len {
                            queue.ui.select(Some(index));
                        }
                    }
                }
            }
        }

        //Seeker
        if let Some((column, row)) = queue.clicked_pos {
            let size = f.size();
            if size.height - 3 == row
                || size.height - 2 == row
                || size.height - 1 == row && column >= 3 && column < size.width - 2
            {
                let ratio = (column - 3) as f64 / size.width as f64;
                let duration = queue.duration().unwrap();

                let new_time = duration * ratio;
                queue.seek_to(new_time);
                queue.play();
            }
            queue.clicked_pos = None;
        }
    }
}

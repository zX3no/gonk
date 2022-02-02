use crate::app::Options;
use gonk_database::Database;
use rodio::DeviceTrait;
use tui::{backend::Backend, layout::*, style::*, widgets::*, Frame};

//TODO: 
//Directory deletion confirmation
//UI to add new directories

pub fn draw<B: Backend>(f: &mut Frame<B>, options: &Options, _db: &Database) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Percentage(50)])
        .split(f.size());

    draw_sound_devices(f, options, chunks[0]);
    draw_dirs(f, options, chunks[1]);
}

pub fn draw_dirs<B: Backend>(f: &mut Frame<B>, options: &Options, chunk: Rect) {
    let items: Vec<_> = options
        .dirs
        .data
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("─Music Directories")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(options.dirs.index);

    f.render_stateful_widget(list, chunk, &mut state);
}

pub fn draw_sound_devices<B: Backend>(f: &mut Frame<B>, options: &Options, chunk: Rect) {
    let default_device = &options.default_device;

    let items: Vec<_> = options
        .devices
        .data
        .iter()
        .map(|device| {
            let name = device.name().expect("Device has no name!");
            if &name == default_device {
                ListItem::new(name)
            } else {
                ListItem::new(name).style(Style::default().add_modifier(Modifier::DIM))
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("─Output Device")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default())
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(options.devices.index);

    f.render_stateful_widget(list, chunk, &mut state);
}

// pub fn draw_prompt<B: Backend>(f: &mut Frame<B>, options: &Options, chunk: Rect) {
//     todo!();
// }

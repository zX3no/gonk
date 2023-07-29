use crate::{widgets::*, Frame};
use gonk_player::{Device, Wasapi};
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders},
};

pub struct Settings {
    pub devices: Vec<Device>,
    pub index: Option<usize>,
    pub current_device: String,
}

impl Settings {
    pub fn new(wanted_device: &str) -> Self {
        let default = Wasapi::default_device();
        let devices = Wasapi::devices();
        let current_device = if devices.iter().any(|device| device.name == wanted_device) {
            wanted_device.to_string()
        } else {
            default.name.to_string()
        };

        Self {
            index: if devices.is_empty() { None } else { Some(0) },
            devices,
            current_device,
        }
    }
}

pub fn selected(settings: &mut Settings) -> Option<&str> {
    if let Some(index) = settings.index {
        if let Some(device) = settings.devices.get(index) {
            return Some(&device.name);
        }
    }
    None
}

pub fn up(settings: &mut Settings) {
    if settings.devices.is_empty() {
        return;
    }

    match settings.index {
        Some(0) => settings.index = Some(settings.devices.len() - 1),
        Some(n) => settings.index = Some(n - 1),
        None => (),
    }
}

pub fn down(settings: &mut Settings) {
    if settings.devices.is_empty() {
        return;
    }

    match settings.index {
        Some(n) if n + 1 < settings.devices.len() => settings.index = Some(n + 1),
        Some(_) => settings.index = Some(0),
        None => (),
    }
}

pub fn draw(settings: &mut Settings, f: &mut Frame, area: Rect) {
    let devices: Vec<&str> = settings
        .devices
        .iter()
        .map(|device| device.name.as_str())
        .collect();

    //TODO: I liked the old item menu bold selections.
    //It doesn't work on most terminals though :(
    let mut items: Vec<ListItem> = devices
        .iter()
        .map(|name| {
            if *name == settings.current_device {
                ListItem::new(Spans::from(vec![
                    Span::styled(
                        ">> ",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::DIM | Modifier::BOLD),
                    ),
                    Span::styled(*name, Style::default().add_modifier(Modifier::BOLD)),
                ]))
            } else {
                ListItem::new(format!("   {name}"))
            }
        })
        .collect();

    if let Some(index) = settings.index {
        let item = items[index]
            .clone()
            .style(Style::default().fg(Color::Black).bg(Color::White));
        items[index] = item;
    }

    let list = List::new(&items)
        .block(
            Block::default()
                .title("─Output Device")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_style(Style::default());

    let mut state = ListState::default();
    state.select(settings.index);

    f.render_stateful_widget(list, area, &mut state);
}

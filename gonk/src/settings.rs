use crate::{widgets::*, Frame, Input};
use gonk_core::Index;
use gonk_player::Player;
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

pub struct Settings {
    pub devices: Index<String>,
    pub current_device: String,
}

impl Settings {
    pub fn new() -> Self {
        let wanted_device = gonk_core::get_output_device();

        let devices: Vec<String> = unsafe {
            gonk_player::devices()
                .into_iter()
                .map(|device| device.name)
                .collect()
        };

        let current_device = if devices.iter().any(|name| name == wanted_device) {
            wanted_device.to_string()
        } else {
            let device = unsafe { gonk_player::default_device() };
            device.name
        };

        Self {
            devices: Index::from(devices),
            current_device,
        }
    }
}

impl Input for Settings {
    fn up(&mut self) {
        self.devices.up();
    }

    fn down(&mut self) {
        self.devices.down();
    }

    fn left(&mut self) {}

    fn right(&mut self) {}
}

pub fn on_enter(settings: &mut Settings, player: &mut Player) {
    if let Some(device) = settings.devices.selected() {
        player.set_output_device(device);
        settings.current_device = device.clone();
    }
}

pub fn draw(settings: &mut Settings, area: Rect, f: &mut Frame) {
    let items: Vec<ListItem> = settings
        .devices
        .data
        .iter()
        .map(|name| {
            if name == &settings.current_device {
                ListItem::new(name.as_str())
            } else {
                ListItem::new(name.as_str()).style(Style::default().add_modifier(Modifier::DIM))
            }
        })
        .collect();

    let list = List::new(&items)
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
    state.select(settings.devices.index());

    f.render_stateful_widget(list, area, &mut state);
}

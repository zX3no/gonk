use crate::{widgets::*, Frame, Input};
use gonk_database::query;
use gonk_player::{Device, DeviceTrait, Index, Player};
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

pub struct Settings {
    pub devices: Index<Device>,
    pub current_device: String,
}

impl Settings {
    pub fn new() -> Self {
        let default_device = Player::default_device();
        let wanted_device = query::playback_device();

        let devices = Player::audio_devices();
        let device_names: Vec<String> = devices.iter().flat_map(DeviceTrait::name).collect();

        let current_device = if !device_names.contains(&wanted_device) {
            let name = default_device.name().unwrap();
            query::set_playback_device(&name);
            name
        } else {
            wanted_device
        };

        Self {
            devices: Index::new(devices, Some(0)),
            current_device,
        }
    }
}

impl Input for Settings {
    fn up(&mut self) {
        self.devices.up();
    }

    fn down(&mut self) {
        self.devices.down()
    }

    fn left(&mut self) {}

    fn right(&mut self) {}
}

pub fn on_enter(settings: &mut Settings, player: &mut Player) {
    if let Some(device) = settings.devices.selected() {
        match player.change_output_device(device) {
            Ok(_) => {
                let name = device.name().unwrap();
                query::set_playback_device(&name);
                settings.current_device = name;
            }
            //TODO: Print error in status bar
            Err(e) => panic!("{:?}", e),
        }
    }
}

#[allow(unused)]
pub fn draw(settings: &mut Settings, area: Rect, f: &mut Frame) {
    let items: Vec<ListItem> = settings
        .devices
        .data
        .iter()
        .map(|device| {
            let name = device.name().unwrap();
            if name == settings.current_device {
                ListItem::new(name)
            } else {
                ListItem::new(name).style(Style::default().add_modifier(Modifier::DIM))
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

use crate::config::Toml;
use crate::widgets::{List, ListItem, ListState};
use crate::Frame;
use gonk_player::{Device, DeviceTrait, Index, Player};
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

pub struct Options {
    pub devices: Index<Device>,
}

impl Options {
    pub fn new(toml: &mut Toml) -> Self {
        let devices = Index::new(Player::output_devices(), Some(0));
        let config_device = toml.output_device();
        let default_device = Player::default_device().unwrap().name().unwrap();

        let mut data: Vec<_> = devices
            .data
            .iter()
            .flat_map(gonk_player::DeviceTrait::name)
            .collect();

        data.retain(|name| name == config_device);

        let device = if data.is_empty() {
            default_device
        } else {
            config_device.to_string()
        };

        toml.set_output_device(device);

        Self { devices }
    }
    pub fn up(&mut self) {
        self.devices.up();
    }
    pub fn down(&mut self) {
        self.devices.down();
    }
    pub fn on_enter(&mut self, player: &mut Player, toml: &mut Toml) {
        if let Some(device) = self.devices.selected() {
            //don't update the device if there is an error
            if player.change_output_device(device) {
                toml.set_output_device(device.name().expect("Device has no name!"));
            }
        }
    }
}

impl Options {
    pub fn draw(&self, area: Rect, f: &mut Frame, toml: &Toml) {
        let default_device = toml.output_device();

        let items: Vec<_> = self
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
        state.select(self.devices.index());

        f.render_stateful_widget(list, area, &mut state);
    }
}

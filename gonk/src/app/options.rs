use crate::toml::Toml;
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
    current_device: String,
}

impl Options {
    pub fn new(toml: &mut Toml) -> Self {
        let default_device = Player::default_device();
        let mut config_device = toml.config.output_device.clone();

        let devices = Player::audio_devices();
        let device_names: Vec<String> = devices.iter().flat_map(DeviceTrait::name).collect();

        if !device_names.contains(&config_device) {
            let name = default_device.name().unwrap();
            config_device = name.clone();
            toml.set_output_device(name);
        }

        Self {
            devices: Index::new(devices, Some(0)),
            current_device: config_device,
        }
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
            match player.change_output_device(device) {
                Ok(_) => {
                    let name = device.name().unwrap();
                    self.current_device = name.clone();
                    toml.set_output_device(name);
                }
                //TODO: Print error in status bar
                Err(e) => panic!("{:?}", e),
            }
        }
    }
}

impl Options {
    pub fn draw(&self, area: Rect, f: &mut Frame) {
        let items: Vec<_> = self
            .devices
            .data
            .iter()
            .map(|device| {
                let name = device.name().unwrap();
                if name == self.current_device {
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

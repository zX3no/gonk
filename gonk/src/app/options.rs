use crate::widget::{List, ListItem, ListState};
use gonk_database::Toml;
use gonk_types::Index;
use rodio::{Device, DeviceTrait, Player};
use tui::{
    backend::Backend,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
    Frame,
};

pub struct Options {
    pub devices: Index<Device>,
    toml: Toml,
}

impl Options {
    pub fn new() -> Self {
        let default_device = Player::default_device()
            .expect("Can't find output device!")
            .name()
            .expect("Device has no name!");

        let devices = Index::new(Player::output_devices(), Some(0));

        let mut toml = Toml::new();
        let config_device = toml.output_device().clone();

        let current_device = if config_device.is_empty() {
            default_device
        } else {
            let mut data: Vec<_> = devices
                .data
                .iter()
                .flat_map(rodio::DeviceTrait::name)
                .collect();

            data.retain(|name| name == &config_device);

            if data.is_empty() {
                default_device
            } else {
                config_device.to_string()
            }
        };

        toml.set_output_device(current_device);

        Self { devices, toml }
    }
    pub fn up(&mut self) {
        self.devices.up();
    }
    pub fn down(&mut self) {
        self.devices.down();
    }
    pub fn on_enter(&mut self, player: &mut Player) {
        if let Some(device) = self.devices.selected() {
            self.toml
                .set_output_device(device.name().expect("Device has no name!"));
            player.change_output_device(device);
        }
    }
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let default_device = self.toml.output_device();

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
        state.select(self.devices.index);

        f.render_stateful_widget(list, f.size(), &mut state);
    }
}

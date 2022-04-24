use crate::{
    app::TOML,
    widget::{List, ListItem, ListState},
    MUT_TOML,
};
use gonk_core::Index;
use gonk_player::{Device, DeviceTrait, Player};
use tui::{
    backend::Backend,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
    Frame,
};

pub struct Options {
    pub devices: Index<Device>,
}

impl Options {
    fn get_current_device(devices: &Index<Device>) -> String {
        let config_device = TOML.output_device();
        let default_device = Player::default_device().name().unwrap();

        let mut data: Vec<_> = devices
            .data
            .iter()
            .flat_map(gonk_player::DeviceTrait::name)
            .collect();

        data.retain(|name| name == config_device);

        if data.is_empty() {
            default_device
        } else {
            config_device.to_string()
        }
    }
    pub fn new() -> Self {
        let devices = Index::new(Player::output_devices(), Some(0));
        let current_device = Self::get_current_device(&devices);

        MUT_TOML
            .fast_write()
            .unwrap()
            .set_output_device(current_device);

        Self { devices }
    }
    pub fn up(&mut self) {
        self.devices.up();
    }
    pub fn down(&mut self) {
        self.devices.down();
    }
    pub fn on_enter(&mut self, player: &mut Player) {
        if let Some(device) = self.devices.selected() {
            //don't update the device if there is an error
            if player.change_output_device(device) {
                MUT_TOML
                    .fast_write()
                    .unwrap()
                    .set_output_device(device.name().expect("Device has no name!"));
            }
        }
    }
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let toml = MUT_TOML.fast_read().unwrap();
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
        state.select(self.devices.index);

        f.render_stateful_widget(list, f.size(), &mut state);
    }
}

use crate::{widgets::*, Frame, Widget};
use crossterm::event::MouseEvent;
use gonk_core::Index;
use gonk_player::{default_device, devices};
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

pub struct Settings {
    pub devices: Index<()>,
    pub current_device: String,
}

impl Settings {
    pub fn new(wanted_device: &str) -> Self {
        let devices = devices();
        let default = default_device().expect("No default output device?");

        let devices: Vec<&'static str> =
            devices.iter().map(|device| device.name.as_str()).collect();

        let current_device = if devices.iter().any(|name| *name == wanted_device) {
            wanted_device.to_string()
        } else {
            default.name.to_string()
        };

        Self {
            devices: Index::default(),
            current_device,
        }
    }

    pub fn selected(&self) -> Option<&str> {
        if let Some(index) = self.devices.index() {
            if let Some(device) = devices().get(index) {
                return Some(&device.name);
            }
        }
        None
    }
}

impl Widget for Settings {
    fn up(&mut self) {
        self.devices.up_with_len(devices().len());
    }

    fn down(&mut self) {
        self.devices.down_with_len(devices().len());
    }

    fn left(&mut self) {}

    fn right(&mut self) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, _: Option<MouseEvent>) {
        draw(self, area, f);
    }
}

pub fn draw(settings: &mut Settings, area: Rect, f: &mut Frame) {
    //TODO: Re-write.
    let mut index = settings.devices.index().unwrap_or(0);
    if index >= devices().len() {
        index = devices().len().saturating_sub(1);
    }
    settings.devices = Index::new(Vec::new(), Some(index));

    let devices: Vec<&str> = devices()
        .iter()
        .map(|device| device.name.as_str())
        .collect();

    let items: Vec<ListItem> = devices
        .iter()
        .map(|name| {
            if *name == settings.current_device {
                ListItem::new(*name).style(Style::default().add_modifier(Modifier::BOLD))
            } else {
                ListItem::new(*name)
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

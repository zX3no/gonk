use gonk_database::Toml;
use gonk_types::Index;
use rodio::{Device, DeviceTrait, Player};

pub enum Mode {
    Directory,
    Device,
}

pub struct Options {
    pub paths: Index<String>,
    pub devices: Index<Device>,
    pub mode: Mode,
    toml: Toml,
}

impl Options {
    pub fn new() -> Self {
        let default_device = Player::default_device()
            .expect("Can't find output device!")
            .name()
            .expect("Device has no name!");

        let devices = Index::new(Player::output_devices(), Some(0));

        let mut toml = Toml::new().unwrap();
        let config_device = toml.output_device();

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
                config_device
            }
        };

        //Update the self.toml file to the correct device
        toml.set_output_device(current_device);

        Self {
            paths: Index::new(toml.paths(), None),
            devices,
            mode: Mode::Device,
            toml,
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            Mode::Directory => {
                if let Some(index) = self.paths.index {
                    if !self.devices.is_empty() && index == 0 {
                        self.mode = Mode::Device;
                        self.paths.select(None);
                        self.devices
                            .select(Some(self.devices.len().saturating_sub(1)));
                        return;
                    }
                    self.paths.up();
                }
            }
            Mode::Device => {
                if let Some(index) = self.devices.index {
                    if !self.paths.is_empty() && index == 0 {
                        self.mode = Mode::Directory;
                        self.devices.select(None);
                        self.paths.select(Some(self.paths.len().saturating_sub(1)));
                        return;
                    }
                }
                self.devices.up();
            }
        }
    }
    pub fn down(&mut self) {
        match self.mode {
            Mode::Directory => {
                if let Some(index) = self.paths.index {
                    if !self.devices.is_empty() && index == self.paths.len().saturating_sub(1) {
                        self.mode = Mode::Device;
                        self.paths.select(None);
                        self.devices.select(Some(0));
                        return;
                    }
                }
                self.paths.down();
            }
            Mode::Device => {
                if let Some(index) = self.devices.index {
                    if !self.paths.is_empty() && index == self.devices.len().saturating_sub(1) {
                        self.mode = Mode::Directory;
                        self.devices.select(None);
                        self.paths.select(Some(0));
                        return;
                    }
                }
                self.devices.down();
            }
        }
    }
    pub fn on_enter(&mut self, player: &mut Player) -> Option<String> {
        match self.mode {
            Mode::Directory => {
                let dir = self.paths.selected().cloned();
                if let Some(dir) = dir {
                    //Delete dir from ui and config file
                    self.toml.remove_path(&dir);
                    self.paths.data.retain(|x| x != &dir);

                    if self.paths.is_empty() {
                        self.paths = Index::new(self.toml.paths(), None);
                        if !self.devices.is_empty() {
                            self.mode = Mode::Device;
                            self.devices.select(Some(0));
                        }
                    } else {
                        self.paths = Index::new(self.toml.paths(), Some(0));
                    }
                    return Some(dir);
                }
            }
            Mode::Device => {
                if let Some(device) = self.devices.selected() {
                    self.toml
                        .set_output_device(device.name().expect("Device has no name!"));
                    player.change_output_device(device);
                }
            }
        }
        None
    }
}

//TODO: Directory deletion confirmation & UI to add new directories
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

impl Options {
    pub fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Percentage(50)])
            .split(f.size());

        self.draw_sound_devices(f, chunks[0]);
        self.draw_dirs(f, chunks[1]);
    }

    pub fn draw_dirs<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        let items: Vec<_> = self
            .paths
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
        state.select(self.paths.index);

        f.render_stateful_widget(list, chunk, &mut state);
    }

    pub fn draw_sound_devices<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) {
        let default_device = self.toml.output_device();

        let items: Vec<_> = self
            .devices
            .data
            .iter()
            .map(|device| {
                let name = device.name().expect("Device has no name!");
                if name == default_device {
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

        f.render_stateful_widget(list, chunk, &mut state);
    }

    // pub fn draw_prompt<B: Backend>(f: &mut Frame<B>, self: &self, chunk: Rect) {
    //     todo!();
    // }
}

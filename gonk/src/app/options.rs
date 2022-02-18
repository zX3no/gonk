use super::Queue;
use crate::index::Index;
use gonk_database::Toml;
use rodio::{Device, DeviceTrait, Player};

pub enum OptionsMode {
    Directory,
    Device,
}

pub struct Options {
    pub paths: Index<String>,
    pub devices: Index<Device>,
    pub mode: OptionsMode,
    //TODO: move colors things here
    // pub colors: Colors,
    // pub hotkeys: Hotkey,
    pub toml: Toml,
    //TODO: move volume here?
}

impl Options {
    pub fn new(mut toml: Toml) -> Self {
        let default_device = Player::default_device()
            .expect("Can't find output device!")
            .name()
            .expect("Device has no name!");

        let devices = Index::new(Player::output_devices(), Some(0));

        let config_device = toml.output_device();

        let current_device = if config_device.is_empty() {
            default_device
        } else {
            let mut data: Vec<_> = devices
                .data
                .iter()
                .flat_map(|device| device.name())
                .collect();
            data.retain(|name| name == &config_device);
            if data.is_empty() {
                default_device
            } else {
                config_device
            }
        };

        //Update the toml file to the correct device
        toml.set_output_device(current_device);

        Self {
            paths: Index::new(toml.paths(), None),
            devices,
            mode: OptionsMode::Device,
            toml,
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            OptionsMode::Directory => {
                if let Some(index) = self.paths.index {
                    if !self.devices.is_empty() && index == 0 {
                        self.mode = OptionsMode::Device;
                        self.paths.select(None);
                        self.devices
                            .select(Some(self.devices.len().saturating_sub(1)));
                        return;
                    }
                    self.paths.up()
                }
            }
            OptionsMode::Device => {
                if let Some(index) = self.devices.index {
                    if !self.paths.is_empty() && index == 0 {
                        self.mode = OptionsMode::Directory;
                        self.devices.select(None);
                        self.paths.select(Some(self.paths.len().saturating_sub(1)));
                        return;
                    }
                }
                self.devices.up()
            }
        }
    }
    pub fn down(&mut self) {
        match self.mode {
            OptionsMode::Directory => {
                if let Some(index) = self.paths.index {
                    if !self.devices.is_empty() && index == self.paths.len().saturating_sub(1) {
                        self.mode = OptionsMode::Device;
                        self.paths.select(None);
                        self.devices.select(Some(0));
                        return;
                    }
                }
                self.paths.down();
            }
            OptionsMode::Device => {
                if let Some(index) = self.devices.index {
                    if !self.paths.is_empty() && index == self.devices.len().saturating_sub(1) {
                        self.mode = OptionsMode::Directory;
                        self.devices.select(None);
                        self.paths.select(Some(0));
                        return;
                    }
                }
                self.devices.down();
            }
        }
    }
    pub fn on_enter(&mut self, queue: &mut Queue) -> Option<String> {
        match self.mode {
            OptionsMode::Directory => {
                let dir = self.paths.selected().cloned();
                if let Some(dir) = dir {
                    //Delete dir from ui and config file
                    self.toml.remove_path(&dir);
                    self.paths.data.retain(|x| x != &dir);

                    if self.paths.is_empty() {
                        self.paths = Index::new(self.toml.paths(), None);
                        if !self.devices.is_empty() {
                            self.mode = OptionsMode::Device;
                            self.devices.select(Some(0));
                        }
                    } else {
                        self.paths = Index::new(self.toml.paths(), Some(0));
                    }
                    return Some(dir);
                }
            }
            OptionsMode::Device => {
                if let Some(device) = self.devices.selected() {
                    self.toml
                        .set_output_device(device.name().expect("Device has no name!"));
                    queue.change_output_device(device);
                }
            }
        }
        None
    }
    pub fn save_volume(&mut self, vol: u16) {
        self.toml.set_volume(vol);
    }
    pub(crate) fn paths(&self) -> &[String] {
        &self.paths.data
    }
}

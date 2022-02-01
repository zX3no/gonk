use crate::index::Index;
use gonk_database::Database;
use rodio::{Device, DeviceTrait, Player};

use super::Queue;

pub enum OptionsMode {
    Directory,
    Device,
}

pub struct Options {
    pub dirs: Index<String>,
    pub default_device: String,
    pub devices: Index<Device>,
    pub mode: OptionsMode,
}

impl Options {
    pub fn new(db: &Database) -> Self {
        Self {
            //TODO: should this be part of struct?
            //can you add a dir while using gronk?
            dirs: Index::new(db.get_music_dirs(), None),
            devices: Index::new(Player::output_devices(), Some(0)),
            default_device: Player::default_device()
                .expect("Can't find output device!")
                .name()
                .expect("Device has no name!"),
            mode: OptionsMode::Device,
        }
    }
    pub fn up(&mut self) {
        match self.mode {
            OptionsMode::Directory => {
                if let Some(index) = self.dirs.index {
                    if index == 0 {
                        self.mode = OptionsMode::Device;
                        self.dirs.select(None);
                        self.devices
                            .select(Some(self.devices.len().saturating_sub(1)));
                        return;
                    }
                }
                self.dirs.up()
            }
            OptionsMode::Device => {
                if let Some(index) = self.devices.index {
                    if index == 0 {
                        self.mode = OptionsMode::Directory;
                        self.devices.select(None);
                        self.dirs.select(Some(self.dirs.len().saturating_sub(1)));
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
                if let Some(index) = self.dirs.index {
                    if index == self.dirs.len().saturating_sub(1) {
                        self.mode = OptionsMode::Device;
                        self.dirs.select(None);
                        self.devices.select(Some(0));
                        return;
                    }
                }
                self.dirs.down();
            }
            OptionsMode::Device => {
                if let Some(index) = self.devices.index {
                    if index == self.devices.len().saturating_sub(1) {
                        self.mode = OptionsMode::Directory;
                        self.devices.select(None);
                        self.dirs.select(Some(0));
                        return;
                    }
                }
                self.devices.down();
            }
        }
    }
    pub fn on_enter(&mut self, queue: &mut Queue) {
        match self.mode {
            OptionsMode::Directory => todo!("prompt user if they want to delete directory"),
            OptionsMode::Device => {
                if let Some(device) = self.devices.selected() {
                    //TODO: Selected device in config file
                    self.default_device = device.name().expect("Device has no name!");
                    queue.change_output_device(device);
                }
            }
        }
    }
}

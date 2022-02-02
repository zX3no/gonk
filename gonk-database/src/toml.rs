use std::fs;

use crate::TOML_DIR;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    paths: Vec<String>,
    output_device: String,
    volume: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Toml {
    config: Config,
}

impl Toml {
    pub fn new() -> std::io::Result<Self> {
        let path = TOML_DIR.as_path();

        let file = if path.exists() {
            fs::read_to_string(path)?
        } else {
            let toml = Toml {
                config: Config {
                    paths: Vec::new(),
                    output_device: String::new(),
                    volume: 15,
                },
            };

            toml::to_string(&toml).unwrap()
        };

        let toml: Toml = toml::from_str(&file)?;

        Ok(toml)
    }
    pub fn volume(&self) -> u16 {
        self.config.volume
    }
    pub fn paths(&self) -> Vec<String> {
        self.config.paths.clone()
    }
    pub fn output_device(&self) -> String {
        self.config.output_device.clone()
    }
    pub fn add_path(&mut self, path: String) {
        if !self.config.paths.contains(&path) {
            self.config.paths.push(path);
            self.write();
        }
    }
    pub fn remove_path(&mut self, path: &str) {
        self.config.paths.retain(|x| x != path);
        self.write();
    }
    pub fn set_volume(&mut self, vol: u16) {
        self.config.volume = vol;
        self.write();
    }
    pub fn set_output_device(&mut self, device: String) {
        self.config.output_device = device;
        self.write();
    }
    pub fn write(&self) {
        let toml = toml::to_string(&self).unwrap();
        fs::write(TOML_DIR.as_path(), toml).unwrap();
    }
}

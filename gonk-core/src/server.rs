use serde::{Deserialize, Serialize};
use std::fs;

use crate::SERVER_CONFIG;

#[derive(Serialize, Deserialize, Default)]
pub struct ServerConfig {
    pub paths: Vec<String>,
    pub ip: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn new() -> Self {
        if SERVER_CONFIG.exists() {
            let file = fs::read_to_string(SERVER_CONFIG.as_path()).unwrap();
            match toml::from_str(&file) {
                Ok(toml) => toml,
                Err(err) => {
                    //TODO: parse and describe error to user?
                    panic!("{:#?}", &err);
                }
            }
        } else {
            let toml = ServerConfig {
                paths: Vec::new(),
                ip: String::from("localhost"),
                #[allow(clippy::zero_prefixed_literal)]
                port: 673,
            };
            toml.write();
            toml
        }
    }
    pub fn write(&self) {
        fs::write(SERVER_CONFIG.as_path(), toml::to_string(&self).unwrap()).unwrap();
    }
    pub fn ip(&self) -> String {
        //TODO: check ip and port for errors
        //https://doc.rust-lang.org/nightly/std/net/struct.SocketAddrV4.html
        format!("{}:{}", self.ip, self.port)
    }
    pub fn add_path(&mut self, path: String) {
        if !self.paths.contains(&path) {
            self.paths.push(path);
            self.write();
        } else {
            println!("Path already added.");
        }
    }
}

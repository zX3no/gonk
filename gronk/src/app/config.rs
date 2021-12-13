use core::panic;
use std::path::PathBuf;

lazy_static! {
    static ref CONFIG_DIR: PathBuf = dirs::config_dir().unwrap();
}

pub struct Config {
    music_dirs: Vec<PathBuf>,
}

impl Config {
    pub fn new() -> Self {
        let args: Vec<_> = std::env::args().skip(1).collect();

        let mut music_dirs = Vec::new();
        if let Some(first) = args.first() {
            if first == "add" {
                if let Some(dir) = args.get(1) {
                    music_dirs.push(PathBuf::from(&dir));
                }
            }
        }

        //TODO: create config dir

        //TODO: create config file
        Self { music_dirs }
    }
    pub fn get_music_dirs(&self) -> &Vec<PathBuf> {
        &self.music_dirs
    }
}

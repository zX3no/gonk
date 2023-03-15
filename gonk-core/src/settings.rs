//! Music player settings
//!
//! Stores the volume, state of the queue and output device
//!
use crate::*;
use std::{
    fs::File,
    io::{BufWriter, Seek, Write},
};

static mut FILE: Lazy<File> = Lazy::new(|| {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(settings_path())
        .unwrap()
});

#[derive(Debug)]
pub struct Settings {
    pub volume: u8,
    pub index: u16,
    pub elapsed: f32,
    pub output_device: String,
    pub music_folder: String,
    pub queue: Vec<Song>,
}

impl Serialize for Settings {
    fn serialize(&self) -> String {
        let mut buffer = String::new();
        buffer.push_str(&self.volume.to_string());
        buffer.push_str(&self.index.to_string());
        buffer.push_str(&self.elapsed.to_string());
        buffer.push_str(&escape(&self.output_device));
        buffer.push_str(&escape(&self.music_folder));
        buffer.push_str(&self.queue.serialize());
        buffer
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 15,
            index: Default::default(),
            elapsed: Default::default(),
            output_device: Default::default(),
            music_folder: Default::default(),
            queue: Default::default(),
        }
    }
}

impl Settings {
    pub fn new() -> Settings {
        match Settings::read() {
            Ok(settings) => settings,
            Err(_) => Settings::default(),
        }
    }

    pub fn read() -> Result<Settings, Box<dyn Error + Send + Sync>> {
        todo!();
    }

    pub fn save(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        unsafe {
            FILE.set_len(0)?;
            FILE.rewind()?;
            let mut writer = BufWriter::new(&*FILE);
            writer.write_all(self.serialize().as_bytes())?;
            writer.flush()?;
        };

        Ok(())
    }
}

//! Music player settings
//!
//! Stores the volume, state of the queue and output device
//!
use crate::*;
use std::{
    fs::File,
    io::{BufWriter, Read, Seek, Write},
};

pub static mut FILE: Option<File> = None;

#[derive(Debug, PartialEq)]
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
        buffer.push('\t');
        buffer.push_str(&self.index.to_string());
        buffer.push('\t');
        buffer.push_str(&self.elapsed.to_string());
        buffer.push('\t');
        buffer.push_str(&escape(&self.output_device));
        buffer.push('\t');
        buffer.push_str(&escape(&self.music_folder));
        buffer.push('\n');
        buffer.push_str(&self.queue.serialize());
        buffer
    }
}

impl Deserialize for Settings {
    type Error = Box<dyn Error>;

    fn deserialize(s: &str) -> Result<Self, Self::Error> {
        let (start, end) = s.split_once('\n').ok_or("Invalid settings")?;
        let split: Vec<&str> = start.split('\t').collect();
        let music_folder = if split.len() == 4 {
            String::new()
        } else {
            split[4].to_string()
        };

        let queue = if end.is_empty() {
            Vec::new()
        } else {
            Vec::<Song>::deserialize(end)?
        };

        Ok(Self {
            volume: split[0].parse::<u8>()?,
            index: split[1].parse::<u16>()?,
            elapsed: split[2].parse::<f32>()?,
            output_device: split[3].to_string(),
            music_folder,
            queue,
        })
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
        unsafe {
            FILE = Some(
                File::options()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(settings_path())
                    .unwrap(),
            );
        }
        match Settings::read() {
            Ok(settings) => settings,
            Err(_) => Settings::default(),
        }
    }

    pub fn read() -> Result<Settings, Box<dyn Error>> {
        unsafe {
            let mut string = String::new();
            FILE.as_ref().unwrap().read_to_string(&mut string)?;
            Settings::deserialize(&string)
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        unsafe {
            FILE.as_ref().unwrap().set_len(0)?;
            FILE.as_ref().unwrap().rewind()?;
            let mut writer = BufWriter::new(&*FILE.as_ref().unwrap());
            writer.write_all(self.serialize().as_bytes())?;
            writer.flush()?;
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings() {
        let settings = Settings::new();
        let string = settings.serialize();
        let s = Settings::deserialize(&string).unwrap();
        assert_eq!(settings, s);
    }
}

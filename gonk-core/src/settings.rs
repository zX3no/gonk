//! Music player settings
//!
//! Stores the volume, state of the queue and output device
//!
use crate::*;
use std::{
    fs::File,
    io::{BufWriter, Read, Seek, Write},
    mem::size_of,
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
        // let bytes = fs::read(settings_path())?;
        let mut bytes = Vec::new();
        let _ = unsafe { FILE.read_to_end(&mut bytes)? };

        let volume = bytes.first().ok_or("Volume is invalid.")?;

        let slice = bytes.get(1..3).ok_or("Index is invalid.")?;
        let index = u16::from_le_bytes(slice.try_into()?);

        let slice = bytes.get(3..7).ok_or("Elapsed is invalid.")?;
        let elapsed = f32::from_le_bytes(slice.try_into()?);

        let slice = bytes.get(7..9).ok_or("Output length device is invalid")?;
        let output_device_len = u16::from_le_bytes(slice.try_into()?) as usize + 9;
        let slice = bytes
            .get(9..output_device_len)
            .ok_or("Ouput device is invalid")?;
        let output_device = from_utf8(slice)?.to_string();

        let slice = bytes
            .get(output_device_len..output_device_len + 2)
            .ok_or("Music folder length is invalid")?;
        let music_folder_len = u16::from_le_bytes(slice.try_into()?) as usize;

        let start = output_device_len + size_of::<u16>();
        let slice = &bytes
            .get(start..start + music_folder_len)
            .ok_or("Music folder is invalid")?;
        let music_folder = from_utf8(slice)?.to_string();

        // let mut queue = Vec::new();
        // let mut i = start + music_folder_len;
        // while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
        //     let song = bytes_to_song(bytes)?;
        //     queue.push(song);
        //     i += SONG_LEN;
        // }

        // Ok(Settings {
        //     index,
        //     volume: *volume,
        //     output_device,
        //     music_folder,
        //     elapsed,
        //     queue,
        // })

        // todo!();
        Ok(Settings::default())
    }

    pub fn save(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        unsafe {
            FILE.set_len(0)?;
            FILE.rewind()?;
        };

        let mut writer = unsafe { BufWriter::new(&*FILE) };

        writer.write_all(&[self.volume])?;
        writer.write_all(&self.index.to_le_bytes())?;
        writer.write_all(&self.elapsed.to_le_bytes())?;

        writer.write_all(&(self.output_device.len() as u16).to_le_bytes())?;
        writer.write_all(self.output_device.as_bytes())?;

        writer.write_all(&(self.music_folder.len() as u16).to_le_bytes())?;
        writer.write_all(self.music_folder.as_bytes())?;

        for song in &self.queue {
            // writer.write_all(&song.to_bytes())?;

            todo!();
        }

        writer.flush()?;

        Ok(())
    }
}

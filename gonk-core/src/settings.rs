use crate::{settings_path, RawSong, SONG_LEN};
use std::{
    fs::File,
    io::{self, BufWriter, Read, Write},
    mem::size_of,
    str::from_utf8_unchecked,
};

// Settings Layout:
//
// Volume
// Index
// u16 (output_device length)
// output_device
// u16 (music_folder length)
// music_folder
// [RawSong]
#[derive(Debug)]
pub struct Settings {
    pub file: File,

    pub volume: u8,
    pub index: u16,
    pub elapsed: f32,
    pub output_device: String,
    pub music_folder: String,
    pub queue: Vec<RawSong>,
}

impl Settings {
    pub unsafe fn new() -> Self {
        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(settings_path())
            .unwrap();
        let mut bytes = Vec::new();
        let Ok(_) = file.read_to_end(&mut bytes) else {
            panic!();
        };

        if bytes.is_empty() {
            Self::default(file)
        } else {
            Self::from_bytes(&bytes, file)
        }
    }
    pub fn default(file: File) -> Self {
        Self {
            file,
            volume: 15,
            index: 0,
            elapsed: 0.0,
            output_device: String::new(),
            music_folder: String::new(),
            queue: Vec::new(),
        }
    }
    pub unsafe fn from_bytes(bytes: &[u8], file: File) -> Self {
        let volume = bytes[0];
        let index = u16::from_le_bytes([bytes[1], bytes[2]]);
        let elapsed = f32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);

        let output_device_len = u16::from_le_bytes([bytes[7], bytes[8]]) as usize + 9;
        if output_device_len >= bytes.len() {
            return Self::default(file);
        }
        let output_device = from_utf8_unchecked(&bytes[9..output_device_len]).to_string();

        let start = output_device_len + size_of::<u16>();
        let music_folder_len =
            u16::from_le_bytes([bytes[output_device_len], bytes[output_device_len + 1]]) as usize;
        if music_folder_len >= bytes.len() {
            return Self::default(file);
        }
        let music_folder = from_utf8_unchecked(&bytes[start..start + music_folder_len]).to_string();

        let mut queue = Vec::new();
        let mut i = start + music_folder_len;
        while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
            queue.push(RawSong::from(bytes));
            i += SONG_LEN;
        }
        Self {
            file,
            index,
            volume,
            output_device,
            music_folder,
            elapsed,
            queue,
        }
    }
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.volume);
        bytes.extend(self.index.to_le_bytes());
        bytes.extend(self.elapsed.to_le_bytes());

        bytes.extend((self.output_device.len() as u16).to_le_bytes());
        bytes.extend(self.output_device.as_bytes());

        bytes.extend((self.music_folder.len() as u16).to_le_bytes());
        bytes.extend(self.music_folder.as_bytes());

        for song in &self.queue {
            bytes.extend(song.as_bytes());
        }
        bytes
    }
    pub fn write(mut self) -> io::Result<Self> {
        self.save()?;
        Ok(self)
    }
    pub fn save(&mut self) -> io::Result<()> {
        self.file.set_len(0).unwrap();
        let mut writer = BufWriter::new(&self.file);

        writer.write_all(&[self.volume])?;
        writer.write_all(&self.index.to_le_bytes())?;
        writer.write_all(&self.elapsed.to_le_bytes())?;

        writer.write_all(&(self.output_device.len() as u16).to_le_bytes())?;
        writer.write_all(self.output_device.as_bytes())?;

        writer.write_all(&(self.music_folder.len() as u16).to_le_bytes())?;
        writer.write_all(self.music_folder.as_bytes())?;

        for song in &self.queue {
            writer.write_all(&song.as_bytes())?;
        }
        Ok(())
    }
}

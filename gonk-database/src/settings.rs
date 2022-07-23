use crate::{db_path, RawSong, SONG_LEN};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
};

pub static mut SETTINGS: Settings = Settings::default();

#[derive(Debug)]
pub struct Settings {
    ///File Handle
    pub writer: Option<BufWriter<File>>,

    ///Data
    pub volume: u8,
    pub queue_position: f32,
    pub queue: Vec<RawSong>,
}

impl Settings {
    pub const fn default() -> Self {
        Self {
            writer: None,
            volume: 0,
            queue_position: 0.0,
            queue: Vec::new(),
        }
    }
    pub fn into_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.volume);
        bytes.extend(self.queue_position.to_le_bytes());
        for song in &self.queue {
            bytes.extend(song.clone().into_bytes());
        }
        bytes
    }
    pub fn from(bytes: Vec<u8>, file: File) -> Self {
        let volume = bytes[0];
        let queue_position = f32::from_le_bytes(bytes[1..5].try_into().unwrap());

        let mut queue = Vec::new();
        let mut i = 5;
        while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
            queue.push(RawSong::from(bytes));
            i += SONG_LEN;
        }

        Self {
            writer: Some(BufWriter::new(file)),
            volume,
            queue_position,
            queue,
        }
    }
}

pub fn init() {
    let path = settings_path();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let bytes = fs::read(&path).unwrap();
    unsafe {
        SETTINGS = Settings::from(bytes, file);
    }
}

pub fn update_volume(new_volume: u8) {
    unsafe {
        SETTINGS.volume = new_volume;
        let bytes = SETTINGS.into_bytes();
        let writer = SETTINGS.writer.as_mut().unwrap();
        writer.write_all(&bytes).unwrap();
        writer.flush().unwrap();
    }
}

pub fn update_queue(queue: Vec<RawSong>, position: f32) {
    unsafe {
        SETTINGS.queue_position = position;
        SETTINGS.queue = queue;
        let bytes = SETTINGS.into_bytes();
        let writer = SETTINGS.writer.as_mut().unwrap();
        writer.write_all(&bytes).unwrap();
        writer.flush().unwrap();
    }
}

fn settings_path() -> PathBuf {
    let mut path = db_path();
    path.pop();
    path.push("settings.db");
    path
}

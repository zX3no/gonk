use crate::{
    db::{bytes_to_song, SONG_LEN},
    *,
};
use std::{
    fs::File,
    io::{BufWriter, Read, Write},
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

#[derive(Debug, Default)]
pub struct Settings {
    pub volume: u8,
    pub index: u16,
    pub elapsed: f32,
    pub output_device: String,
    pub music_folder: String,
    pub queue: Vec<Song>,
}

impl Settings {
    pub fn new() -> Settings {
        match Settings::read() {
            Ok(settings) => settings,
            //TODO: File related error migth show up?
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

        let mut queue = Vec::new();
        let mut i = start + music_folder_len;
        while let Some(bytes) = bytes.get(i..i + SONG_LEN) {
            let song = bytes_to_song(bytes)?;
            queue.push(song);
            i += SONG_LEN;
        }

        Ok(Settings {
            index,
            volume: *volume,
            output_device,
            music_folder,
            elapsed,
            queue,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let mut bytes = Vec::new();
        bytes.push(self.volume);
        bytes.extend(self.index.to_le_bytes());
        bytes.extend(self.elapsed.to_le_bytes());

        bytes.extend((self.output_device.len() as u16).to_le_bytes());
        bytes.extend(self.output_device.as_bytes());

        bytes.extend((self.music_folder.len() as u16).to_le_bytes());
        bytes.extend(self.music_folder.as_bytes());

        for song in &self.queue {
            bytes.extend(song.to_bytes());
        }
        unsafe { FILE.set_len(0)? };
        let mut writer = unsafe { BufWriter::new(&*FILE) };

        writer.write_all(&[self.volume])?;
        writer.write_all(&self.index.to_le_bytes())?;
        writer.write_all(&self.elapsed.to_le_bytes())?;

        writer.write_all(&(self.output_device.len() as u16).to_le_bytes())?;
        writer.write_all(self.output_device.as_bytes())?;

        writer.write_all(&(self.music_folder.len() as u16).to_le_bytes())?;
        writer.write_all(self.music_folder.as_bytes())?;

        for song in &self.queue {
            writer.write_all(&song.to_bytes())?;
        }

        Ok(())
    }
}

/*

pub fn save_volume(new_volume: u8) {
    unsafe {
        DB.settings.volume = new_volume;
        DB.settings.save().unwrap();
    }
}

pub fn save_queue(queue: &[Song], index: u16, elapsed: f32) {
    unsafe {
        DB.settings.queue = queue.iter().map(RawSong::from).collect();
        DB.settings.index = index;
        DB.settings.elapsed = elapsed;
        DB.settings.save().unwrap();
    };
}

pub fn update_queue_state(index: u16, elapsed: f32) {
    unsafe {
        DB.settings.elapsed = elapsed;
        DB.settings.index = index;
        DB.settings.save().unwrap();
    }
}

pub fn update_output_device(device: &str) {
    unsafe {
        DB.settings.output_device = device.to_string();
        DB.settings.save().unwrap();
    }
}

pub fn update_music_folder(folder: &str) {
    unsafe {
        DB.settings.music_folder = folder.replace('\\', "/");
        DB.settings.save().unwrap();
    }
}

pub fn get_saved_queue() -> (Vec<Song>, Option<usize>, f32) {
    let settings = unsafe { &DB.settings };
    let index = if settings.queue.is_empty() {
        None
    } else {
        Some(settings.index as usize)
    };

    (
        settings
            .queue
            .iter()
            .map(|song| Song::from(&song.as_bytes()))
            .collect(),
        index,
        settings.elapsed,
    )
}

pub fn output_device() -> &'static str {
    unsafe { &DB.settings.output_device }
}

pub fn music_folder() -> &'static str {
    unsafe { &DB.settings.music_folder }
}

pub fn volume() -> u8 {
    unsafe { DB.settings.volume }
}

*/

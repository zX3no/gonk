use crate::{
    db::{bytes_to_song, song_to_bytes, SONG_LEN},
    *,
};
use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    mem::size_of,
};

static mut FILE: Lazy<File> = Lazy::new(|| File::open(settings_path()).unwrap());

#[derive(Debug)]
pub struct Settings {
    pub volume: u8,
    pub index: u16,
    pub elapsed: f32,
    pub output_device: String,
    pub music_folder: String,
    pub queue: Vec<Song>,
}

pub fn new() -> Result<Settings, Box<dyn Error + Send + Sync>> {
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

pub fn save(settings: &Settings) -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    bytes.push(settings.volume);
    bytes.extend(settings.index.to_le_bytes());
    bytes.extend(settings.elapsed.to_le_bytes());

    bytes.extend((settings.output_device.len() as u16).to_le_bytes());
    bytes.extend(settings.output_device.as_bytes());

    bytes.extend((settings.music_folder.len() as u16).to_le_bytes());
    bytes.extend(settings.music_folder.as_bytes());

    for song in &settings.queue {
        let song = song_to_bytes(
            &song.artist,
            &song.album,
            &song.title,
            &song.path,
            song.track_number,
            song.disc_number,
            song.gain,
        );
        bytes.extend(song);
    }
    unsafe { FILE.set_len(0)? };
    let mut writer = unsafe { BufWriter::new(&*FILE) };

    writer.write_all(&[settings.volume])?;
    writer.write_all(&settings.index.to_le_bytes())?;
    writer.write_all(&settings.elapsed.to_le_bytes())?;

    writer.write_all(&(settings.output_device.len() as u16).to_le_bytes())?;
    writer.write_all(settings.output_device.as_bytes())?;

    writer.write_all(&(settings.music_folder.len() as u16).to_le_bytes())?;
    writer.write_all(settings.music_folder.as_bytes())?;

    for song in &settings.queue {
        let bytes = song_to_bytes(
            &song.artist,
            &song.album,
            &song.title,
            &song.path,
            song.track_number,
            song.disc_number,
            song.gain,
        );
        writer.write_all(&bytes)?;
    }

    Ok(())
}

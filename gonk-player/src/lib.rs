//! TODO: Describe the audio backend
//!
//! Pipewire is currently not implemented because WSL doesn't work well with audio.
//! AND...I don't have a spare drive to put linux on.
#![feature(const_float_bits_conv, lazy_cell)]

pub mod decoder;
pub mod wasapi;

use gonk_core::{Index, Song};
pub use wasapi::{default_device, devices, imm_device_enumerator, init, Device, Wasapi};

pub const VOLUME_REDUCTION: f32 = 150.0;

use crossbeam_queue::SegQueue;
use makepad_windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
use mini::*;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU8, Ordering},
        Condvar, LazyLock, Mutex,
    },
    thread,
    time::Duration,
};

use crate::decoder::Symphonia;

pub struct AtomicF32 {
    storage: AtomicU32,
}

impl AtomicF32 {
    pub const fn new(value: f32) -> Self {
        let as_u64 = value.to_bits();
        Self {
            storage: AtomicU32::new(as_u64),
        }
    }
    pub fn store(&self, value: f32, ordering: Ordering) {
        let as_u32 = value.to_bits();
        self.storage.store(as_u32, ordering)
    }
    pub fn load(&self, ordering: Ordering) -> f32 {
        let as_u32 = self.storage.load(ordering);
        f32::from_bits(as_u32)
    }
}

//Two threads should always be running
//The wasapi thread and the decoder thread.
//The wasapi thread reads from a buffer, if the buffer is empty, block until it's not.
//The decoder thread needs a way to request a new file to be read.
//It should read the contents of the audio file into the shared buffer.

//TODO: Need to figure out how the buffer is going to work.
//This is not working properly.
// pub static mut BUFFER: Option<boxcar::Vec<f32>> = None;

use jagged::vector::Vector;

pub static mut BUFFER: LazyLock<Vector<f32>> = LazyLock::new(|| Vector::new());
pub static mut ELAPSED: Duration = Duration::from_secs(0);
pub static mut DURATION: Duration = Duration::from_secs(0);
pub static mut SEEK: Option<f32> = None;
pub static mut VOLUME: AtomicF32 = AtomicF32::new(15.0 / VOLUME_REDUCTION);

//TODO: Thread safety?
pub static mut OUTPUT_DEVICE: Option<Device> = None;

static mut EVENTS: SegQueue<Event> = SegQueue::new();

#[derive(Debug, PartialEq)]
enum Event {
    Stop,
    TogglePlayback,
    Song(PathBuf),
}

//TODO:
//Song queue
//Next/Previous
//Mute

pub unsafe fn start_decoder_thread() {
    thread::spawn(|| {
        info!("Spawned decoder thread!");

        let mut sym: Option<Symphonia> = None;
        let mut path: Option<PathBuf> = None;
        let mut paused = false;

        loop {
            match EVENTS.pop() {
                Some(Event::Song(new_path)) => {
                    info!("{}", new_path.display());
                    path = Some(new_path);
                }
                Some(Event::Stop) => {}
                //TODO: This doesn't work because the decoding could be done.
                //Then samples would be loaded from the WASAPI thread.
                Some(Event::TogglePlayback) => {
                    paused = !paused;
                }
                _ => {}
            }

            let Some(path) = &path else {
                continue;
            };

            let sym = if let Some(sym) = &mut sym {
                sym
            } else {
                sym = Some(Symphonia::new(path).unwrap());
                sym.as_mut().unwrap()
            };

            //TODO: We need to cache every packet length and the timestamp.
            //This way we can calculate the current time but getting the buffer index.
            //Seeking will need to be compeletely redesigned.
            //When seeking with symphonia the mediasourcestream will be updated.
            //This would break my buffer. If the packet is already loaded I want to use it.
            if let Some(packet) = sym.next_packet() {
                BUFFER.extend(packet.samples().to_vec());

                if let Some(seek) = SEEK {
                    info!(
                        "Seeking {} / {}",
                        seek as u32,
                        DURATION.as_secs_f32() as u32
                    );
                    todo!();
                    sym.seek(seek);
                    SEEK = None;
                }

                if let Some(device) = &OUTPUT_DEVICE {
                    info!("Changing output device {}", device.name);
                    OUTPUT_DEVICE = None;
                }

                ELAPSED = sym.elapsed();
            } else {
                todo!();
            }
        }
    });
}

//TODO: Handle seeking. `i` is unchanged. This is wrong because the buffer is cleared.
pub unsafe fn start_wasapi_thread(device: Device) {
    thread::spawn(move || {
        // info!("Spawned WASAPI thread!");
        // let default = default_device();
        let wasapi = Wasapi::new(&device, Some(44100)).unwrap();
        let block_align = wasapi.format.Format.nBlockAlign as u32;
        let mut i = 0;

        loop {
            std::hint::spin_loop();
            //Sample-rate probably changed if this fails.
            let padding = wasapi.audio_client.GetCurrentPadding().unwrap();
            let buffer_size = wasapi.audio_client.GetBufferSize().unwrap();

            let n_frames = buffer_size - 1 - padding;
            assert!(n_frames < buffer_size - padding);

            let size = (n_frames * block_align) as usize;

            if size == 0 {
                continue;
            }

            let b = wasapi.render_client.GetBuffer(n_frames).unwrap();
            let output = std::slice::from_raw_parts_mut(b, size);
            let channels = wasapi.format.Format.nChannels as usize;
            let volume = VOLUME.load(Ordering::Relaxed);

            macro_rules! next {
                () => {
                    if let Some(s) = (*BUFFER).get(i) {
                        let s = (s * volume).to_le_bytes();
                        i += 1;
                        s
                    } else {
                        (0.0f32).to_le_bytes()
                    }
                };
            }

            for bytes in output.chunks_mut(std::mem::size_of::<f32>() * channels) {
                bytes[0..4].copy_from_slice(&next!());
                if channels > 1 {
                    bytes[4..8].copy_from_slice(&next!());
                }
            }

            wasapi.render_client.ReleaseBuffer(n_frames, 0).unwrap();

            if WaitForSingleObject(wasapi.event, u32::MAX) != WAIT_OBJECT_0 {
                unreachable!()
            }
        }
    });
}

pub fn get_volume() -> u8 {
    unsafe { (VOLUME.load(Ordering::Relaxed) * VOLUME_REDUCTION) as u8 }
}

pub fn set_volume(volume: u8) {
    unsafe {
        VOLUME.store(volume as f32 / VOLUME_REDUCTION, Ordering::Relaxed);
    }
}

pub fn volume_up() {
    unsafe {
        let volume = (VOLUME.load(Ordering::Relaxed) * VOLUME_REDUCTION + 5.0).clamp(0.0, 100.0)
            / VOLUME_REDUCTION;
        VOLUME.store(volume, Ordering::Relaxed);
    }
}

pub fn volume_down() {
    unsafe {
        let volume = (VOLUME.load(Ordering::Relaxed) * VOLUME_REDUCTION - 5.0).clamp(0.0, 100.0)
            / VOLUME_REDUCTION;
        VOLUME.store(volume, Ordering::Relaxed);
    }
}

pub fn toggle_playback() {
    unsafe { EVENTS.push(Event::TogglePlayback) }
}

pub fn seek(pos: f32) {
    unsafe {
        SEEK = Some(pos);
    }
}

pub fn seek_foward() {
    unsafe {
        let pos = (ELAPSED.as_secs_f32() + 10.0).clamp(0.0, f32::MAX);
        SEEK = Some(pos);
    }
}

pub fn seek_backward() {
    unsafe {
        let pos = (ELAPSED.as_secs_f32() - 10.0).clamp(0.0, f32::MAX);
        SEEK = Some(pos);
    }
}

//TODO: I don't know how I'm going to handle playing files.
pub fn play<P: AsRef<Path>>(path: P) {
    unsafe {
        EVENTS.push(Event::Song(path.as_ref().to_path_buf()));
    }
}

pub fn play_song(song: &Song) {
    unsafe {
        // PATH = Mutex::new(Some(PathBuf::from(&song.path)));
        // if song.gain != 0.0 {
        //     VOLUME.store(song.gain / VOLUME_REDUCTION, Ordering::Relaxed);
        // }
        // PATH_CONDVAR.notify_all();
    }
}

pub fn stop() {
    // unsafe { COMMAND.store(STOP, Ordering::Relaxed) };
}

pub fn set_output_device(device: &str) {
    let d = devices();
    unsafe {
        match d.into_iter().find(|d| d.name == device) {
            Some(device) => OUTPUT_DEVICE = Some(device),
            None => panic!(
                "Could not find {} in {:?}",
                device,
                devices()
                    .into_iter()
                    .map(|d| d.name)
                    .collect::<Vec<String>>()
            ),
        }
    }
}

pub fn clear_queue(songs: &mut Index<Song>) {
    songs.clear();
    // unsafe { COMMAND.store(STOP, Ordering::Relaxed) };
}

pub fn play_index(songs: &mut Index<Song>, i: usize) {
    songs.select(Some(i));
    if let Some(song) = songs.selected() {
        play(&song.path);
    }
}

pub fn delete(songs: &mut Index<Song>, index: usize) {
    if songs.is_empty() {
        return;
    }

    songs.remove(index);

    if let Some(playing) = songs.index() {
        let len = songs.len();
        if len == 0 {
            *songs = Index::default();
            stop();
        } else if index == playing && index == 0 {
            songs.select(Some(0));
            if let Some(song) = songs.selected() {
                play(&song.path);
            }
        } else if index == playing && index == len {
            songs.select(Some(len - 1));
            if let Some(song) = songs.selected() {
                play(&song.path);
            }
        } else if index < playing {
            songs.select(Some(playing - 1));
        }
    };
}

pub fn clear_except_playing(songs: &mut Index<Song>) {
    if let Some(index) = songs.index() {
        let playing = songs.remove(index);
        *songs = Index::new(vec![playing], Some(0));
    }
}

pub fn is_playing() -> bool {
    todo!()
    // unsafe { COMMAND.load(Ordering::Relaxed) == EMPTY }
}

pub fn elapsed() -> Duration {
    unsafe { ELAPSED }
}

pub fn duration() -> Duration {
    unsafe { DURATION }
}

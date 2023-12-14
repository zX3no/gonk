//! TODO: Describe the audio backend
//!
//! Pipewire is currently not implemented because WSL doesn't work well with audio.
//! AND...I don't have a spare drive to put linux on.
#![feature(const_float_bits_conv, lazy_cell)]
#![allow(unused)]

pub mod decoder;
pub mod wasapi;

use gonk_core::{Index, Song};
use ringbuf::StaticRb;
use symphonia::core::audio::SampleBuffer;
pub use wasapi::{default_device, devices, imm_device_enumerator, Device, Wasapi};

pub const VOLUME_REDUCTION: f32 = 150.0;

use crossbeam_queue::SegQueue;
use makepad_windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
use mini::*;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
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

// pub static mut BUFFER: LazyLock<Vector<f32>> = LazyLock::new(|| Vector::new());
pub static mut ELAPSED: Duration = Duration::from_secs(0);
pub static mut DURATION: Duration = Duration::from_secs(0);
pub static mut SEEK: Option<f32> = None;
pub static mut VOLUME: AtomicF32 = AtomicF32::new(15.0 / VOLUME_REDUCTION);

//TODO: Thread safety?
static mut OUTPUT_DEVICE: Option<Device> = None;
static mut EVENTS: SegQueue<Event> = SegQueue::new();

const RB_SIZE: usize = 4096;

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

pub fn spawn_audio_threads(device: Device) {
    unsafe {
        let rb = StaticRb::<f32, RB_SIZE>::default();
        let (mut prod, mut cons) = rb.split();

        thread::spawn(move || {
            info!("Spawned decoder thread!");

            let mut sym: Option<Symphonia> = None;
            let mut paused = false;

            let mut packet: Option<SampleBuffer<f32>> = None;
            let mut i = 0;

            loop {
                std::thread::sleep(std::time::Duration::from_millis(1));

                match EVENTS.pop() {
                    Some(Event::Song(new_path)) => {
                        info!("{}", new_path.display());
                        sym = Some(Symphonia::new(&new_path).unwrap());
                    }
                    Some(Event::Stop) => {
                        sym = None;
                    }
                    //TODO: This doesn't work because the decoding could be done.
                    //Then samples would be loaded from the WASAPI thread.
                    Some(Event::TogglePlayback) => {
                        paused = !paused;
                    }
                    _ => {}
                }

                let Some(sym) = &mut sym else {
                    continue;
                };

                if let Some(p) = &mut packet {
                    i += prod.push_slice(&p.samples()[i..]);
                    if i == p.len() {
                        i = 0;
                        packet = None;
                    }

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
                    packet = sym.next_packet();
                }
            }
        });

        thread::spawn(move || {
            info!("Spawned WASAPI thread!");
            let wasapi = Wasapi::new(&device, Some(44100)).unwrap();
            let block_align = wasapi.format.Format.nBlockAlign as u32;

            //TODO: This thread is spinning too much!
            loop {
                if cons.is_empty() {
                    // std::thread::sleep(std::time::Duration::from_nanos(22));
                    // std::hint::spin_loop();
                    continue;
                }

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

                for bytes in output.chunks_mut(std::mem::size_of::<f32>() * channels) {
                    let Some(sample) = cons.pop() else {
                        break;
                    };

                    bytes[0..4].copy_from_slice(&(sample * volume).to_le_bytes());

                    if channels > 1 {
                        let Some(sample) = cons.pop() else {
                            break;
                        };

                        bytes[4..8].copy_from_slice(&(sample * volume).to_le_bytes());
                    }
                }

                wasapi.render_client.ReleaseBuffer(n_frames, 0).unwrap();

                if WaitForSingleObject(wasapi.event, u32::MAX) != WAIT_OBJECT_0 {
                    unreachable!()
                }
            }
        });
    }
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

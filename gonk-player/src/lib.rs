#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]
use gonk_core::{Index, Song};
use std::fs::File;
use std::io::ErrorKind;
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatReader, Track};
use symphonia::{
    core::{
        audio::SampleBuffer,
        codecs::{Decoder, DecoderOptions},
        formats::{FormatOptions, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::Time,
    },
    default::get_probe,
};

mod wasapi;
pub use wasapi::*;

const VOLUME: f32 = 300.0;

#[allow(unused)]
#[inline]
pub fn sleep(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

#[inline]
pub fn calc_volume(volume: u8) -> f32 {
    volume as f32 / VOLUME
}

#[derive(Debug)]
pub enum Event {
    OutputDevice(String),
}

#[derive(Default)]
pub struct Queue<T> {
    data: Arc<Mutex<VecDeque<T>>>,
    condvar: Arc<Condvar>,
    capacity: usize,
}

impl<T> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Default::default(),
            condvar: Default::default(),
            capacity,
        }
    }
    ///Blocks when the queue is full.
    pub fn push(&self, t: T) {
        let mut data = self.data.lock().unwrap();
        while data.len() > self.capacity {
            data = self.condvar.wait(data).unwrap();
        }
        data.push_back(t);
        self.condvar.notify_one();
    }
    ///Always unblocks the sender.
    pub fn pop(&self) -> Option<T> {
        self.condvar.notify_one();
        self.data.lock().unwrap().pop_front()
    }
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().is_empty()
    }
    pub fn clear(&mut self) {
        self.data.lock().unwrap().clear();
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            condvar: self.condvar.clone(),
            capacity: self.capacity,
        }
    }
}

pub struct State {
    pub playing: bool,
    pub finished: bool,
    pub elapsed: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub gain: f32,
    pub path: String,
    pub device: String,
}

//Nooooo! You can't use shared mutable state, it...it's too unsafe!
static mut STATE: State = State {
    playing: false,
    finished: false,
    elapsed: Duration::from_secs(0),
    duration: Duration::from_secs(0),
    volume: 15.0 / VOLUME,
    gain: 0.0,
    path: String::new(),
    device: String::new(),
};

pub struct Player {
    pub volume: u8,
    pub songs: Index<Song>,
}

impl Player {
    pub unsafe fn new(device: &str, volume: u8, songs: Index<Song>, elapsed: f32) -> Self {
        update_devices();

        let devices = devices();
        let default = default_device().unwrap();
        let d = devices.iter().find(|d| d.name == device);
        let mut device = if let Some(d) = d { d } else { default };

        let selected = songs.selected().cloned();

        thread::spawn(move || {
            let elapsed = Duration::from_secs_f32(elapsed);
            let mut sample_rate = 44100;
            let mut path = String::new();

            STATE.elapsed = elapsed;
            STATE.volume = calc_volume(volume);

            if let Some(song) = &selected {
                match Symphonia::new(&song.path) {
                    Ok(mut sym) => {
                        sym.seek(elapsed);

                        if sym.sample_rate() != sample_rate {
                            sample_rate = sym.sample_rate();
                        }

                        path = song.path.clone();
                        STATE.gain = song.gain;
                        STATE.duration = sym.duration();
                        STATE.playing = false;
                        STATE.finished = false;
                        STATE.path = song.path.clone();
                        SYMPHONIA = Some(sym);
                    }
                    Err(err) => gonk_core::log!("Failed to restore queue. {}", err),
                }
            }

            let mut handle = StreamHandle::new(device, sample_rate);

            loop {
                //Clear the sample buffer if nothing is playing.
                //Sometimes old samples would get left over,
                //clearing them avoids clicks and pops.
                if SYMPHONIA.is_none() && !handle.queue.is_empty() {
                    handle.queue.clear();
                }

                if path != STATE.path {
                    path = STATE.path.clone();
                    match Symphonia::new(&path) {
                        Ok(sym) => {
                            if sym.sample_rate() != sample_rate {
                                sample_rate = sym.sample_rate();
                                if handle.set_sample_rate(sample_rate).is_err() {
                                    handle = StreamHandle::new(device, sample_rate);
                                }
                            }

                            STATE.duration = sym.duration();
                            STATE.playing = true;
                            STATE.finished = false;
                            SYMPHONIA = Some(sym);
                        }
                        Err(err) => gonk_core::log!("Failed to play song. {}", err),
                    }
                }

                if device.name != STATE.device {
                    let d = devices.iter().find(|device| device.name == STATE.device);
                    if let Some(d) = d {
                        device = d;
                        handle = StreamHandle::new(device, sample_rate);
                    }
                }

                if let Some(d) = &mut SYMPHONIA {
                    if STATE.playing && !STATE.finished {
                        if let Some(next) = d.next_packet() {
                            for smp in next.samples() {
                                if STATE.gain == 0.0 {
                                    //Reduce the volume a little to match
                                    //songs with replay gain information.
                                    handle.queue.push(smp * STATE.volume * 0.75);
                                } else {
                                    handle.queue.push(smp * STATE.volume * STATE.gain);
                                }
                            }
                        } else {
                            STATE.finished = true;
                        }
                    }
                }
            }
        });

        Self { volume, songs }
    }
    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 5).clamp(0, 100);
        unsafe {
            STATE.volume = calc_volume(self.volume);
        }
    }
    pub fn volume_down(&mut self) {
        self.volume = (self.volume as i8 - 5).clamp(0, 100) as u8;
        unsafe {
            STATE.volume = calc_volume(self.volume);
        }
    }
    pub fn toggle_playback(&self) {
        unsafe { STATE.playing = !STATE.playing };
    }
    pub fn play(&self, path: &str, gain: f32) {
        unsafe {
            STATE.path = path.to_string();
            STATE.gain = gain;
        }
    }
    pub fn seek_foward(&mut self) {
        let pos = (self.elapsed().as_secs_f64() + 10.0).clamp(0.0, f64::MAX);
        let pos = Duration::from_secs_f64(pos);
        self.seek(pos);
    }
    pub fn seek_backward(&mut self) {
        let pos = (self.elapsed().as_secs_f64() - 10.0).clamp(0.0, f64::MAX);
        let pos = Duration::from_secs_f64(pos);
        self.seek(pos);
    }
    pub fn seek(&mut self, pos: Duration) {
        match unsafe { &mut SYMPHONIA } {
            Some(sym) => sym.seek(pos),
            None => (),
        };
    }
    pub fn elapsed(&self) -> Duration {
        unsafe { STATE.elapsed }
    }
    pub fn duration(&self) -> Duration {
        unsafe { STATE.duration }
    }
    pub fn is_playing(&self) -> bool {
        unsafe { STATE.playing }
    }
    pub fn next(&mut self) {
        self.songs.down();
        if let Some(song) = self.songs.selected() {
            self.play(&song.path, song.gain)
        }
    }
    pub fn prev(&mut self) {
        self.songs.up();
        if let Some(song) = self.songs.selected() {
            self.play(&song.path, song.gain)
        }
    }
    pub fn delete_index(&mut self, i: usize) -> Result<(), String> {
        if self.songs.is_empty() {
            return Ok(());
        }
        self.songs.data.remove(i);

        if let Some(playing) = self.songs.index() {
            let len = self.songs.len();

            if len == 0 {
                self.clear();
            } else if i == playing && i == 0 {
                if i == 0 {
                    self.songs.select(Some(0));
                }
                return self.play_index(self.songs.index().unwrap());
            } else if i == playing && i == len {
                self.songs.select(Some(len - 1));
            } else if i < playing {
                self.songs.select(Some(playing - 1));
            }
        };
        Ok(())
    }
    pub fn clear(&mut self) {
        unsafe {
            STATE.playing = false;
            STATE.finished = false;
            SYMPHONIA = None;
        };
        self.songs = Index::default();
    }
    pub fn clear_except_playing(&mut self) {
        if let Some(index) = self.songs.index() {
            let playing = self.songs.data.remove(index);
            self.songs = Index::new(vec![playing], Some(0));
        }
    }
    pub fn add(&mut self, songs: &[Song]) -> Result<(), String> {
        self.songs.data.extend(songs.to_vec());
        if self.songs.selected().is_none() {
            self.songs.select(Some(0));
            self.play_index(0)?;
        }
        Ok(())
    }
    pub fn play_index(&mut self, i: usize) -> Result<(), String> {
        self.songs.select(Some(i));
        if let Some(song) = self.songs.selected() {
            self.play(&song.path, song.gain);
        }
        Ok(())
    }
    ///Checks if the current song is finished. If yes, the next song is played.
    pub fn check_next(&mut self) {
        unsafe {
            if STATE.finished {
                STATE.finished = false;
                self.next();
            }
        }
    }
    pub fn set_output_device(&self, device: &str) {
        unsafe {
            STATE.device = device.to_string();
        }
    }
}

static mut SYMPHONIA: Option<Symphonia> = None;

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: Track,
    elapsed: u64,
    duration: u64,
    error_count: u8,
}

impl Symphonia {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let probed = get_probe()
            .format(
                &Hint::default(),
                mss,
                &FormatOptions {
                    prebuild_seek_index: true,
                    seek_index_fill_rate: 1,
                    enable_gapless: false,
                },
                &MetadataOptions::default(),
            )
            .unwrap();

        let track = probed.format.default_track().unwrap().to_owned();

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap();

        let n_frames = track.codec_params.n_frames.unwrap();
        let duration = track.codec_params.start_ts + n_frames;

        Ok(Self {
            format_reader: probed.format,
            decoder,
            track,
            duration,
            elapsed: 0,
            error_count: 0,
        })
    }
    pub fn elapsed(&self) -> Duration {
        let tb = self.track.codec_params.time_base.unwrap();
        let time = tb.calc_time(self.elapsed);
        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
    }
    pub fn duration(&self) -> Duration {
        let tb = self.track.codec_params.time_base.unwrap();
        let time = tb.calc_time(self.duration);
        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
    }
    pub fn sample_rate(&self) -> u32 {
        self.track.codec_params.sample_rate.unwrap()
    }
    pub fn seek(&mut self, pos: Duration) {
        match self.format_reader.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time: Time::new(pos.as_secs(), pos.subsec_nanos() as f64 / 1_000_000_000.0),
                track_id: None,
            },
        ) {
            Ok(_) => (),
            Err(_) => unsafe {
                STATE.finished = true;
            },
        }
    }
    pub fn next_packet(&mut self) -> Option<SampleBuffer<f32>> {
        let next_packet = match self.format_reader.next_packet() {
            Ok(next_packet) => {
                self.error_count = 0;
                next_packet
            }
            Err(err) => match err {
                Error::IoError(err) => match err.kind() {
                    ErrorKind::Other if err.to_string() == "EOF" => {
                        self.elapsed = self.duration;
                        return None;
                    }
                    _ => panic!("{}", err),
                },
                Error::SeekError(_) | Error::DecodeError(_) => {
                    self.error_count += 1;
                    if self.error_count > 2 {
                        return None;
                    }
                    return self.next_packet();
                }
                _ => panic!("{}", err),
            },
        };

        self.elapsed = next_packet.ts();
        unsafe { STATE.elapsed = self.elapsed() };

        let decoded = self.decoder.decode(&next_packet).unwrap();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        Some(buffer)
    }
}

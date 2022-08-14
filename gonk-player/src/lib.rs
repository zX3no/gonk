#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]
use gonk_core::{Index, Song};
use std::fs::File;
use std::sync::mpsc::{self, Sender};
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

#[macro_export]
macro_rules! unwrap_or_return {
    ( $e:expr ) => {
        unsafe {
            match &mut $e {
                Some(x) => x,
                None => return,
            }
        }
    };
}

#[allow(unused)]
#[inline]
fn sleep(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

#[derive(Default)]
pub struct Queue<T> {
    q: Arc<Mutex<VecDeque<T>>>,
    cv: Arc<Condvar>,
    capacity: usize,
}

impl<T> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            q: Default::default(),
            cv: Default::default(),
            capacity,
        }
    }
    pub fn push(&self, t: T) {
        let mut lq = self.q.lock().unwrap();
        while lq.len() > self.capacity {
            lq = self.cv.wait(lq).unwrap();
        }
        lq.push_back(t);
        self.cv.notify_one();
    }
    pub fn pop(&self) -> T {
        let mut lq = self.q.lock().unwrap();
        while lq.len() == 0 {
            lq = self.cv.wait(lq).unwrap();
        }
        self.cv.notify_one();
        lq.pop_front().unwrap()
    }
    pub fn len(&self) -> usize {
        self.q.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.q.lock().unwrap().is_empty()
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Self {
            q: self.q.clone(),
            cv: self.cv.clone(),
            capacity: self.capacity,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Volume(u8),
    Play(String),
    OutputDevice(String),
}

#[inline]
pub fn calc_volume(volume: u8) -> f32 {
    volume as f32 / 1500.0
}

pub struct State {
    pub playing: bool,
    pub finished: bool,
    pub elapsed: Duration,
    pub duration: Duration,
}

static mut STATE: State = State {
    playing: false,
    finished: false, //This triggers the next song
    elapsed: Duration::from_secs(0),
    duration: Duration::from_secs(0),
};

pub struct Player {
    pub s: Sender<Event>,
    pub volume: u8,
    pub songs: Index<Song>,
}

impl Player {
    pub fn new(device: &str, volume: u8, songs: Index<Song>, elapsed: f32) -> Self {
        let devices = unsafe { devices() };
        let d = devices.iter().find(|d| d.name == device);
        let mut device = if let Some(d) = d {
            d.clone()
        } else {
            unsafe { default_device() }
        };

        let (s, r) = mpsc::channel();

        let mut sample_rate = 44100;
        let mut handle = unsafe { create_stream(&device, sample_rate) };
        let mut vol = calc_volume(volume);

        if let Some(song) = songs.selected() {
            //This is slow >100ms in a debug build.
            match Symphonia::new(&song.path) {
                Ok(mut sym) => {
                    let pos = Duration::from_secs_f32(elapsed);
                    sym.seek(pos);

                    let sr = sym.sample_rate();
                    if sr != sample_rate {
                        sample_rate = sr;
                        handle = unsafe { create_stream(&device, sample_rate) };
                    }
                    unsafe {
                        STATE.elapsed = pos;
                        STATE.duration = sym.duration();
                        STATE.playing = false;
                        STATE.finished = false;
                        SYMPHONIA = Some(sym);
                    };
                }
                Err(err) => gonk_core::log!("Failed to restore queue. {}", err),
            }
        }

        thread::spawn(move || loop {
            if let Ok(event) = r.try_recv() {
                match event {
                    Event::Volume(v) => {
                        let v = v.clamp(0, 100);
                        vol = calc_volume(v);
                    }
                    Event::Play(path) => match Symphonia::new(&path) {
                        Ok(sym) => {
                            let sr = sym.sample_rate();
                            if sr != sample_rate {
                                sample_rate = sr;
                                handle = unsafe { create_stream(&device, sample_rate) };
                            }
                            unsafe {
                                STATE.duration = sym.duration();
                                STATE.playing = true;
                                STATE.finished = false;
                                SYMPHONIA = Some(sym);
                            };
                        }
                        Err(err) => gonk_core::log!("Failed to play song. {}", err),
                    },
                    Event::OutputDevice(name) => {
                        let d = devices.iter().find(|device| device.name == name);
                        if let Some(d) = d {
                            device = d.clone();
                            handle = unsafe { create_stream(&device, sample_rate) };
                        }
                    }
                }
            }

            unsafe {
                if let Some(d) = &mut SYMPHONIA {
                    if STATE.playing && !STATE.finished {
                        if let Some(next) = d.next_packet() {
                            for smp in next.samples() {
                                handle.queue.push(smp * vol)
                            }
                        } else {
                            STATE.finished = true;
                        }
                    }
                }
            }
        });

        Self { s, volume, songs }
    }
    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 5).clamp(0, 100);
        self.s.send(Event::Volume(self.volume)).unwrap();
    }
    pub fn volume_down(&mut self) {
        self.volume = (self.volume as i8 - 5).clamp(0, 100) as u8;
        self.s.send(Event::Volume(self.volume)).unwrap();
    }
    pub fn toggle_playback(&self) {
        unsafe { STATE.playing = !STATE.playing }
    }
    pub fn play(&self, path: &str) {
        self.s.send(Event::Play(path.to_string())).unwrap();
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
        let sym = unwrap_or_return!(SYMPHONIA);
        sym.seek(pos);
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
            self.play(&song.path)
        }
    }
    pub fn prev(&mut self) {
        self.songs.up();
        if let Some(song) = self.songs.selected() {
            self.play(&song.path)
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
            self.play(&song.path);
        }
        Ok(())
    }
    pub fn update(&mut self) -> bool {
        unsafe {
            if STATE.finished {
                STATE.finished = false;
                self.next();
                true
            } else {
                false
            }
        }
    }
    pub fn set_output_device(&self, device: &str) {
        self.s
            .send(Event::OutputDevice(device.to_string()))
            .unwrap();
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
                    std::io::ErrorKind::UnexpectedEof => {
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

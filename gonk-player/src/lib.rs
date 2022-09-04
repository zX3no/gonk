#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]
use gonk_core::{Index, Song};
use std::fs::File;
use std::io::ErrorKind;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
use std::{collections::VecDeque, sync::Arc, thread, time::Duration};
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

#[cfg(windows)]
mod wasapi;

#[cfg(windows)]
pub use wasapi::*;

#[cfg(unix)]
mod pipewire;

#[cfg(unix)]
pub use pipewire::*;

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
    data: Arc<RwLock<VecDeque<T>>>,
    capacity: Arc<AtomicUsize>,
}

impl<T> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Default::default(),
            capacity: Arc::new(AtomicUsize::new(capacity)),
        }
    }
    ///Blocks when the queue is full.
    pub fn push(&self, t: T) {
        while self.len() > self.capacity.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_nanos(100));
        }
        self.data.write().unwrap().push_back(t);
    }
    pub fn pop(&self) -> Option<T> {
        self.data.write().unwrap().pop_front()
    }
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }
    pub fn clear(&mut self) {
        self.data.write().unwrap().clear();
    }
    pub fn resize(&mut self, capacity: usize) {
        self.capacity.store(capacity, Ordering::Relaxed);
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            capacity: self.capacity.clone(),
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
    pub sample_rate: u32,
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
    sample_rate: 44100,
};

static mut SYMPHONIA: Option<Symphonia> = None;
static mut STREAM: Option<StreamHandle> = None;

pub struct Player {
    pub volume: u8,
    pub songs: Index<Song>,
}

impl Player {
    //TODO: This is probably a good example of why you shouldn't use global state.
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

            STATE.elapsed = elapsed;
            STATE.volume = calc_volume(volume);

            if let Some(song) = &selected {
                match Symphonia::new(&song.path) {
                    Ok(mut sym) => {
                        sym.seek(elapsed);

                        if sym.sample_rate() != sample_rate {
                            sample_rate = sym.sample_rate();
                        }

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

            STREAM = Some(StreamHandle::new(device, sample_rate));
            let stream = STREAM.as_mut().unwrap();

            loop {
                if device.name != STATE.device {
                    if let Some(d) = devices.iter().find(|device| device.name == STATE.device) {
                        device = d;
                        *stream = StreamHandle::new(device, sample_rate);
                    }
                }

                if STATE.sample_rate != sample_rate {
                    sample_rate = STATE.sample_rate;
                    if stream.set_sample_rate(sample_rate).is_err() {
                        *stream = StreamHandle::new(device, sample_rate);
                    }
                }

                //Half the volume to match songs without replay gain information.
                let gain = if STATE.gain == 0.0 { 0.50 } else { STATE.gain };

                if let Some(sym) = &mut SYMPHONIA {
                    if STATE.playing && !STATE.finished {
                        if let Some(next) = sym.next_packet() {
                            for smp in next.samples() {
                                stream.queue.push(smp * STATE.volume * gain);
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
        unsafe {
            STATE.playing = !STATE.playing;
            if let Some(stream) = &mut STREAM {
                if STATE.playing {
                    stream.play();
                } else {
                    stream.pause();
                }
            }
        }
    }
    pub fn play_song(&self, path: &str, gain: f32) {
        unsafe {
            STATE.path = path.to_string();
            STATE.gain = gain;
            STATE.elapsed = Duration::default();

            match Symphonia::new(path) {
                Ok(sym) => {
                    STATE.sample_rate = sym.sample_rate();
                    STATE.duration = sym.duration();
                    STATE.finished = false;
                    SYMPHONIA = Some(sym);

                    //TODO: This get's rid of the lag but causes clicking.
                    if let Some(stream) = &mut STREAM {
                        stream.queue.clear();

                        //Play if paused.
                        if !STATE.playing {
                            STATE.playing = true;
                            stream.play();
                        }
                    }
                }
                Err(err) => gonk_core::log!("Failed to play song. {}", err),
            }
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
            self.play_song(&song.path, song.gain)
        }
    }
    pub fn prev(&mut self) {
        self.songs.up();
        if let Some(song) = self.songs.selected() {
            self.play_song(&song.path, song.gain)
        }
    }
    pub fn delete_index(&mut self, i: usize) {
        if self.songs.is_empty() {
            return;
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
                self.play_index(self.songs.index().unwrap());
            } else if i == playing && i == len {
                self.songs.select(Some(len - 1));
            } else if i < playing {
                self.songs.select(Some(playing - 1));
            }
        };
    }
    pub fn clear(&mut self) {
        unsafe {
            STATE.playing = false;
            STATE.finished = false;
            STATE.path = String::new();
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
    pub fn add(&mut self, songs: &[Song]) {
        self.songs.data.extend(songs.to_vec());
        if self.songs.selected().is_none() {
            self.songs.select(Some(0));
            self.play_index(0);
        }
    }
    pub fn play_index(&mut self, i: usize) {
        self.songs.select(Some(i));
        if let Some(song) = self.songs.selected() {
            self.play_song(&song.path, song.gain);
        }
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

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: Track,
    elapsed: u64,
    duration: u64,
    error_count: u8,
}

impl Symphonia {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let probed = get_probe().format(
            &Hint::default(),
            mss,
            &FormatOptions {
                prebuild_seek_index: true,
                seek_index_fill_rate: 1,
                enable_gapless: false,
            },
            &MetadataOptions::default(),
        )?;

        let track = probed.format.default_track().ok_or("")?.to_owned();
        let n_frames = track.codec_params.n_frames.ok_or("")?;
        let duration = track.codec_params.start_ts + n_frames;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

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
        if decoded.frames() == 0 {
            panic!("NO FRAMES!");
        }
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        Some(buffer)
    }
}

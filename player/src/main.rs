#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]
use gonk_database::{Index, Song};
use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};
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

#[derive(Debug, PartialEq)]
pub enum Event {
    TogglePlayback,
    Stop,
    Volume(u8),
    Seek(f64),
    Play(String),
}

#[inline]
pub fn calc_volume(volume: u8) -> f32 {
    volume as f32 / 500.0
}

pub struct State {
    pub playing: bool,
    pub finished: bool,
    pub elapsed: Duration,
    pub duration: Duration,
}

static mut STATE: State = State {
    playing: false,
    finished: false,
    elapsed: Duration::from_secs(0),
    duration: Duration::from_secs(0),
};

pub struct Player {
    pub s: Sender<Event>,
    pub volume: u8,
    pub songs: Index<Song>,
}

impl Player {
    pub fn new(volume: u8) -> Self {
        let (s, r) = mpsc::channel();

        //TODO: Cleanly drop the stream handle
        thread::spawn(move || {
            let mut decoder: Option<Symphonia> = None;
            let mut sample_rate = 44100;
            let mut handle = unsafe { create_stream(sample_rate) };
            let mut playing = false;
            let mut finished = false;
            let mut volume = calc_volume(volume);

            loop {
                if let Ok(event) = r.try_recv() {
                    match event {
                        Event::TogglePlayback => playing = !playing,
                        Event::Volume(v) => {
                            let v = v.clamp(0, 100);
                            volume = calc_volume(v);
                        }
                        Event::Seek(pos) => {
                            let pos = Duration::from_secs_f64(pos);
                            if let Some(decoder) = &mut decoder {
                                decoder.seek(pos);
                            }
                        }
                        Event::Play(path) => {
                            let d = Symphonia::new(path);
                            let sr = d.sample_rate();
                            if sr != sample_rate {
                                sample_rate = sr;
                                handle = unsafe { create_stream(sample_rate) };
                            }
                            decoder = Some(d);
                            playing = true;
                            finished = false;
                        }
                        Event::Stop => {
                            playing = false;
                            finished = true;
                            decoder = None;
                        }
                    }
                }

                if let Some(d) = &mut decoder {
                    //Update the player state.
                    let state = State {
                        playing,
                        finished,
                        elapsed: d.elapsed(),
                        duration: d.duration(),
                    };
                    unsafe { STATE = state };

                    if playing {
                        if let Some(next) = d.next_packet() {
                            for smp in next.samples() {
                                handle.queue.push(smp * volume)
                            }
                        } else {
                            //Song is finished
                            decoder = None;
                            finished = true;
                        }
                    }
                }
            }
        });

        Self {
            s,
            volume,
            songs: Index::default(),
        }
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
        self.s.send(Event::TogglePlayback).unwrap();
    }
    pub fn play(&self, path: &str) {
        self.s.send(Event::Play(path.to_string())).unwrap();
    }
    pub fn seek_by(&self, duration: Duration) {
        let pos = (self.elapsed() + duration).as_secs_f64();
        self.s.send(Event::Seek(pos)).unwrap();
    }
    pub fn seek_to(&self, position: Duration) {
        self.s.send(Event::Seek(position.as_secs_f64())).unwrap();
    }
    pub fn elapsed(&self) -> Duration {
        unsafe { STATE.elapsed }
    }
    pub fn duration(&self) -> Duration {
        unsafe { STATE.duration }
    }
    pub fn playing(&self) -> bool {
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
    pub fn clear(&mut self) {
        self.s.send(Event::Stop).unwrap();
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
            self.play_selected()?;
        }
        Ok(())
    }
    pub fn play_selected(&mut self) -> Result<(), String> {
        if let Some(song) = self.songs.selected() {
            if Path::new(&song.path).exists() {
                self.play(&song.path);
            } else {
                return Err(format!("Path does not exist: {:?}", song.path));
            }
        }
        Ok(())
    }
}

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: Track,
    elapsed: u64,
    duration: Duration,
}

impl Symphonia {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let file = File::open(path).unwrap();
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

        let tb = track.codec_params.time_base.unwrap();
        let n_frames = track.codec_params.n_frames.unwrap();
        let dur = track.codec_params.start_ts + n_frames;
        let time = tb.calc_time(dur);
        let duration = Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac);

        Self {
            format_reader: probed.format,
            decoder,
            track,
            duration,
            elapsed: 0,
        }
    }
    pub fn elapsed(&self) -> Duration {
        let tb = self.track.codec_params.time_base.unwrap();
        let time = tb.calc_time(self.elapsed);
        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn sample_rate(&self) -> u32 {
        self.track.codec_params.sample_rate.unwrap()
    }
    pub fn seek(&mut self, pos: Duration) {
        self.format_reader
            .seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time: Time::new(pos.as_secs(), pos.subsec_nanos() as f64 / 1_000_000_000.0),
                    track_id: None,
                },
            )
            .unwrap();
    }
    pub fn next_packet(&mut self) -> Option<SampleBuffer<f32>> {
        let next_packet = match self.format_reader.next_packet() {
            Ok(next_packet) => next_packet,
            Err(err) => {
                if self.elapsed() == self.duration() {
                    return None;
                }
                panic!("{}", err);
            }
        };

        let ts = next_packet.ts();
        //This is probably the last packet.
        if ts < self.elapsed {
            let n_frames = self.track.codec_params.n_frames.unwrap();
            let dur = self.track.codec_params.start_ts + n_frames;
            self.elapsed = dur;
        } else {
            self.elapsed = ts;
        }

        let decoded = self.decoder.decode(&next_packet).unwrap();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        Some(buffer)
    }
}

fn main() {
    let mut player = Player::new(15);

    let _path = r"D:\OneDrive\Music\Foxtails\fawn\09. life is a death scene, princess.flac";
    let path = r"D:\OneDrive\Music\Foxtails\fawn\06. gallons of spiders went flying thru the stratosphere.flac";
    player.play(path);
    player.clear();
    player.play(path);

    thread::park();
}

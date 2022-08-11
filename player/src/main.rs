#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case,
    unused
)]
use std::fs::File;
use std::path::{Path, PathBuf};
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
        errors::{Error, SeekErrorKind},
        formats::{FormatOptions, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::{Hint, ProbeResult},
        units::{Time, TimeBase},
    },
    default::get_probe,
};

mod wasapi;
pub use wasapi::*;

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
    State(State),
    Volume(u16),
    Seek(f64),
    Play(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Playing,
    Paused,
}

const VOLUME_STEP: u8 = 5;
const VOLUME_REDUCTION: f32 = 500.0;

struct Player {
    s: Sender<Event>,
}

impl Player {
    pub fn new() -> Self {
        let (s, r) = mpsc::channel();

        Self { s }
    }
}

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: Track,
    elapsed: u64,
}

impl Symphonia {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let file = File::open(path).unwrap();
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut probed = get_probe()
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

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap();

        Self {
            format_reader: probed.format,
            decoder,
            track,
            elapsed: 0,
        }
    }
    pub fn elapsed(&self) -> Duration {
        let tb = self.track.codec_params.time_base.unwrap();
        let time = tb.calc_time(self.elapsed);
        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
    }
    pub fn duration(&self) -> Duration {
        let tb = self.track.codec_params.time_base.unwrap();
        let n_frames = self.track.codec_params.n_frames.unwrap();
        let dur = self.track.codec_params.start_ts + n_frames;
        let time = self
            .track
            .codec_params
            .time_base
            .unwrap()
            .calc_time(n_frames);
        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
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
    pub fn next_packet(&mut self) -> SampleBuffer<f32> {
        let next_packet = match self.format_reader.next_packet() {
            Ok(next_packet) => next_packet,
            Err(err) => {
                panic!("{}", err);
            }
        };

        let tb = self.track.codec_params.time_base.unwrap();
        let ts = next_packet.ts();
        self.elapsed = ts;

        let decoded = self.decoder.decode(&next_packet).unwrap();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        buffer
    }
}

fn main() {
    #[rustfmt::skip]
    let path = r"D:\OneDrive\Music\Foxtails\fawn\09. life is a death scene, princess.flac";
    // let path = r"D:\OneDrive\Music\Nirvana\Nevermind (Remastered 2021)\12. Nirvana - Something In The Way (Remastered 2021).flac";
    let mut decoder = Symphonia::new(path);

    let (s, r) = mpsc::channel();

    thread::spawn(move || {
        let handle = unsafe { create_stream(decoder.sample_rate()) };
        let mut state = State::Playing;
        let mut volume = 15.0 / VOLUME_REDUCTION;

        loop {
            if let Ok(event) = r.try_recv() {
                match event {
                    Event::State(s) => state = s,
                    Event::Volume(v) => {
                        volume = v as f32 / VOLUME_REDUCTION;
                    }
                    Event::Seek(pos) => {
                        let pos = Duration::from_secs_f64(pos);
                        decoder.seek(pos);
                    }
                    Event::Play(path) => {}
                }
            }

            if state != State::Paused {
                let next_packet = decoder.next_packet();
                for smp in next_packet.samples() {
                    handle.queue.push(smp * volume)
                }
            }
        }
    });

    s.send(Event::Seek(95.0)).unwrap();

    thread::park();
}

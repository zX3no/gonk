#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case,
    unused
)]
use std::fs::File;
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};
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

fn decode() {
    let file =
        File::open(r"D:\OneDrive\Music\Foxtails\fawn\09. life is a death scene, princess.flac")
            .unwrap();
    // let file = File::open(r"D:\OneDrive\Music\Nirvana\Nevermind (Remastered 2021)\12. Nirvana - Something In The Way (Remastered 2021).flac").unwrap();
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

    let track = probed.format.default_track().unwrap();
    let _sample_rate = track.codec_params.sample_rate.unwrap() as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .unwrap();

    let handle = unsafe { create_stream() };
    loop {
        let next_packet = match probed.format.next_packet() {
            Ok(next_packet) => next_packet,
            Err(_) => {
                std::thread::park();
                panic!();
            }
        };
        let decoded = decoder.decode(&next_packet).unwrap();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        let buffer = buffer.samples().iter();

        for smp in buffer {
            handle.queue.push(smp * 0.03)
        }
    }
}

fn main() {
    // decode();
    let mut handle = unsafe { create_stream() };

    let mut phase: f32 = 0.0;
    let pitch: f32 = 440.0;
    let gain: f32 = 0.1;
    let step = std::f32::consts::PI * 2.0 * pitch / handle.sample_rate as f32;

    loop {
        let smp = phase.sin() * gain;
        phase += step;
        if phase >= std::f32::consts::PI * 2.0 {
            phase -= std::f32::consts::PI * 2.0
        }

        handle.queue.push(smp * 0.03);
    }
}

//! Decoder for audio files.
//!
//! The problem is as follows:
//!
//! Reading packets from a file is slow when using a mutex.
//! Packets are not a consistant size and can only be read in chunks.
//! Because they are read in chunks they need to be put in a buffer when they are read.
//!
//! Idealy a packet would be read than slowly the samples from that packet would be pushed into the buffer.
//!
//! Now I think about it maybe the buffer could auto-resize based on packet length.
//! Nevermind the latency is too high when setting the buffer size according to packet length.
//! Really you should be able to have a buffer size less than the packet length.
//!
//! The duration needs to be read frequently to stay up to date.
//!
//!
//!
//!
use crate::{State, ELAPSED, STATE};
use gonk_core::Lazy;
use std::io::ErrorKind;
use std::time::Duration;
use std::{collections::VecDeque, fs::File, path::Path};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatReader, Track};
use symphonia::{
    core::{
        audio::SampleBuffer,
        codecs,
        formats::{FormatOptions, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::Time,
    },
    default::get_probe,
};

//TODO: This doesn't work becuase the capacity will need to be set to hold at least a packet
//This is not stable plus it's a very large number of samples.
#[derive(Default)]
pub struct Buffer {
    inner: VecDeque<f32>,
    capacity: usize,
    pub average_packet_size: usize,
}

impl Buffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: VecDeque::new(),
            capacity,
            average_packet_size: 0,
        }
    }
    pub fn is_full(&self) -> bool {
        self.inner.len() + self.average_packet_size > self.capacity
    }
    pub fn pop(&mut self) -> Option<f32> {
        self.inner.pop_front()
    }
    pub fn push(&mut self, slice: &[f32]) {
        if slice.len() > self.capacity {
            self.capacity = (slice.len() as f32 * 1.1) as usize;
        }
        self.average_packet_size += slice.len();
        self.average_packet_size /= 2;

        self.inner.extend(slice);
    }
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
    }
}

pub static mut BUFFER: Lazy<Buffer> = Lazy::new(Buffer::default);

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn codecs::Decoder>,
    track: Track,
    elapsed: u64,
    duration: u64,
    error_count: u8,
}

impl Symphonia {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
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
            .make(&track.codec_params, &codecs::DecoderOptions::default())?;

        // let millis = 200;
        // let sample_rate = track.codec_params.sample_rate.unwrap() as usize;
        // let channels = track.codec_params.channels.unwrap().count();
        // let capacity = ((millis * sample_rate) / 1000) * channels;

        // unsafe { BUFFER.set_capacity(capacity) };

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
    pub fn sample_rate(&self) -> usize {
        self.track.codec_params.sample_rate.unwrap() as usize
    }
    pub fn channels(&self) -> usize {
        self.track.codec_params.channels.unwrap().count()
    }
    //TODO: I would like seeking out of bounds to play the next song.
    //I can't trust symphonia to provide accurate errors so it's not worth the hassle.
    //I could use pos + elapsed > duration but the duration isn't accurate.
    pub fn seek(&mut self, pos: f32) {
        let pos = Duration::from_secs_f32(pos);

        //Ignore errors.
        let _ = self.format_reader.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time: Time::new(pos.as_secs(), pos.subsec_nanos() as f64 / 1_000_000_000.0),
                track_id: None,
            },
        );
    }
    pub fn next_packet(&mut self) -> Option<SampleBuffer<f32>> {
        if self.error_count > 2 || unsafe { &STATE } == &State::Finished {
            return None;
        }

        let next_packet = match self.format_reader.next_packet() {
            Ok(next_packet) => {
                self.error_count = 0;
                next_packet
            }
            Err(err) => match err {
                Error::IoError(e) if e.kind() == ErrorKind::UnexpectedEof => {
                    //Just in case my 250ms addition is not enough.
                    if self.elapsed() + Duration::from_secs(1) > self.duration() {
                        unsafe { STATE = State::Finished };
                        return None;
                    } else {
                        self.error_count += 1;
                        return self.next_packet();
                    }
                }
                _ => {
                    gonk_core::log!("{}", err);
                    self.error_count += 1;
                    return self.next_packet();
                }
            },
        };

        self.elapsed = next_packet.ts();
        unsafe { ELAPSED = self.elapsed() };

        //HACK: Sometimes the end of file error does not indicate the end of the file?
        //The duration is a little bit longer than the maximum elapsed??
        //The final packet will make the elapsed time move backwards???
        if self.elapsed() + Duration::from_millis(250) > self.duration() {
            unsafe { STATE = State::Finished };
            return None;
        }

        match self.decoder.decode(&next_packet) {
            Ok(decoded) => {
                let mut buffer =
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
                buffer.copy_interleaved_ref(decoded);
                Some(buffer)
            }
            Err(err) => {
                gonk_core::log!("{}", err);
                self.error_count += 1;
                self.next_packet()
            }
        }
    }
}

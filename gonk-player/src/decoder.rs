//! Decoder for audio files.
//!
//! Packets can only be loaded in full not partially.
//! A `VecDeque` stores the excess.
//!
//! Decoding on the audio thread is probably not ideal.
//! Actually the whole application runs on the same thread, including the audio.
//!
//! It's simple and works...surprisingly.
use crate::State;
use gonk_core::profile;
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

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn codecs::Decoder>,
    track: Track,
    elapsed: u64,
    duration: u64,
    error_count: u8,

    buffer: VecDeque<f32>,
    capacity: usize,
}

impl Symphonia {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        profile!();
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

        let track = probed.format.default_track().ok_or("track")?.to_owned();
        let n_frames = track.codec_params.n_frames.ok_or("n_frames")?;
        let duration = track.codec_params.start_ts + n_frames;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &codecs::DecoderOptions::default())?;

        let millis = 20;
        let sample_rate = track.codec_params.sample_rate.ok_or("sample_rate")? as usize;
        let channels = track.codec_params.channels.ok_or("channels")?.count();
        let capacity = ((millis * sample_rate) / 1000) * channels;

        Ok(Self {
            format_reader: probed.format,
            decoder,
            track,
            duration,
            elapsed: 0,
            error_count: 0,
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        })
    }
    pub fn is_full(&self) -> bool {
        self.buffer.len() > self.capacity
    }
    pub fn pop(&mut self) -> Option<f32> {
        self.buffer.pop_front()
    }
    pub fn push(&mut self, slice: &[f32]) {
        self.buffer.extend(slice);
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
    pub fn next_packet(
        &mut self,
        elapsed: &mut Duration,
        state: &mut State,
    ) -> Option<SampleBuffer<f32>> {
        if self.error_count > 2 || state == &State::Finished {
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
                        *state = State::Finished;
                        return None;
                    } else {
                        self.error_count += 1;
                        return self.next_packet(elapsed, state);
                    }
                }
                _ => {
                    gonk_core::log!("{}", err);
                    self.error_count += 1;
                    return self.next_packet(elapsed, state);
                }
            },
        };

        self.elapsed = next_packet.ts();
        *elapsed = self.elapsed();

        //HACK: Sometimes the end of file error does not indicate the end of the file?
        //The duration is a little bit longer than the maximum elapsed??
        //The final packet will make the elapsed time move backwards???
        if self.elapsed() + Duration::from_millis(250) > self.duration() {
            *state = State::Finished;
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
                self.next_packet(elapsed, state)
            }
        }
    }
}

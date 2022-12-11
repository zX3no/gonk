//! Decoder for audio files.
//!
//! A sample buffer of 20ms is filled for audio backends to use.
//!
use crate::{State, ELAPSED, STATE};
use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};
use rb::{Consumer, Producer, RbProducer, SpscRb, RB};
use ringbuf::SharedRb;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{fs::File, path::Path};
use std::{io::ErrorKind, thread};
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

pub struct Decoder {
    symphonia: Arc<RwLock<Symphonia>>,
    cons: Consumer<f32>,
    // prod: Producer<f32>,
}

impl Decoder {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let symphonia = Symphonia::new(path)?;
        let ring_len = ((20 * symphonia.sample_rate() as usize) / 1000) * symphonia.channels();
        // let rb = ringbuf::HeapRb::<f32>::new(ring_len);
        // let (mut prod, cons) = rb.split();

        let rb = SpscRb::new(ring_len);
        let (prod, cons) = (rb.producer(), rb.consumer());

        let symphonia = Arc::new(RwLock::new(symphonia));
        let sym = symphonia.clone();

        //Constantly fill the buffer with new samples.
        thread::spawn(move || loop {
            // thread::sleep(Duration::from_millis(50));
            let Some(packet) = sym.write().unwrap().next_packet() else {
                continue;
            };
            // eprintln!("write");

            prod.write_blocking(packet.samples()).unwrap();
        });

        Ok(Self {
            symphonia,
            cons,
            // prod,
        })
    }
    //I hate needing to do this...
    pub fn duration(&self) -> Duration {
        self.symphonia.read().unwrap().duration()
    }
    pub fn sample_rate(&self) -> u32 {
        self.symphonia.read().unwrap().sample_rate()
    }
    pub fn refill(&mut self) {
        // if !self.overflow.is_empty() {
        //     // self.prod.write_()
        // }

        // let mut sym = self.symphonia.write().unwrap();

        // let Some(packet) = sym.next_packet() else {
        //     return;
        // };

        // if self.prod.write(packet.samples()).is_err() {
        //     self.overflow.extend_from_slice(packet.samples());
        // };
    }
}

impl Deref for Decoder {
    type Target = Consumer<f32>;

    fn deref(&self) -> &Self::Target {
        &self.cons
    }
}

impl DerefMut for Decoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cons
    }
}

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

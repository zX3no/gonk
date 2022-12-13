//! Decoder for audio files.
//!
//! A sample buffer of 20ms is filled for audio backends to use.
//!
use crate::{State, ELAPSED, STATE};
use rb::{RbProducer, SpscRb, RB};
use std::{
    fs::File,
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
};
use std::{io::ErrorKind, thread};
use std::{ptr::addr_of_mut, time::Duration};
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
    pub prod: rb::Producer<f32>,
    pub cons: rb::Consumer<f32>,

    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn codecs::Decoder>,
    track: Track,
    elapsed: u64,
    duration: u64,
    error_count: u8,

    trigger: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    condvar: Arc<Condvar>,
}

impl Symphonia {
    pub fn new(path: impl AsRef<Path>) -> Result<Pin<Box<Self>>, Box<dyn std::error::Error>> {
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

        let millis = 20;
        let sample_rate = track.codec_params.sample_rate.unwrap() as usize;
        let channels = track.codec_params.channels.unwrap().count();
        let ring_len = ((millis * sample_rate) / 1000) * channels;

        let rb = SpscRb::new(ring_len);
        let (prod, cons) = (rb.producer(), rb.consumer());

        let mut sym = Box::pin(Self {
            prod,
            cons,
            format_reader: probed.format,
            decoder,
            track,
            duration,
            elapsed: 0,
            error_count: 0,
            trigger: Arc::new(AtomicBool::new(false)),
            handle: None,
            condvar: Arc::new(Condvar::new()),
        });

        let ptr: usize = addr_of_mut!(sym) as usize;
        let trigger = sym.trigger.clone();
        let condvar = sym.condvar.clone();

        //This will probably need a condvar or something.
        let handle = thread::spawn(move || {
            let sym = ptr as *mut Symphonia;

            while !trigger.load(Ordering::Relaxed) {
                // thread::sleep(Duration::from_millis(2));
                if let Some(packet) = unsafe { (*sym).next_packet() } {
                    for sample in packet.samples() {
                        //Wait until we can fit the packet into the buffer.
                        unsafe { (*sym).prod.write_blocking(&[*sample]).unwrap() };
                    }
                }
            }
        });

        sym.handle = Some(handle);

        Ok(sym)
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

impl Drop for Symphonia {
    fn drop(&mut self) {
        self.trigger.store(true, Ordering::Relaxed);
        while !self.handle.as_ref().unwrap().is_finished() {}
        dbg!("finished");
    }
}

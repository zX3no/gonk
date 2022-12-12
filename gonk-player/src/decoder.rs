//! Decoder for audio files.
//!
//! A sample buffer of 20ms is filled for audio backends to use.
//!
use crate::{State, ELAPSED, STATE};
use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::addr_of_mut,
};
use ringbuf::HeapRb;
use std::sync::Arc;
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

type Consumer = ringbuf::Consumer<f32, Arc<ringbuf::SharedRb<f32, Vec<MaybeUninit<f32>>>>>;

pub fn new_leak(
    path: impl AsRef<Path>,
) -> Result<&'static mut Decoder, Box<dyn std::error::Error>> {
    let symphonia = Symphonia::new(path)?;

    let millis = 20;
    let ring_len = ((millis * symphonia.sample_rate()) / 1000) * symphonia.channels();
    let rb = HeapRb::<f32>::new(ring_len);
    let (mut prod, cons) = rb.split();

    let mut boxed = Box::new(Decoder { symphonia, cons });

    let cast: usize = addr_of_mut!(boxed.symphonia) as usize;

    thread::spawn(move || {
        let sym = cast as *mut Symphonia;
        loop {
            if let Some(packet) = unsafe { (*sym).next_packet() } {
                for sample in packet.samples() {
                    //Wait until we can fit the packet into the buffer.
                    loop {
                        match prod.push(*sample) {
                            Ok(_) => break,
                            Err(_) => continue,
                        }
                    }
                }
            }
        }
    });

    Ok(Box::leak(boxed))
}

pub fn new(path: impl AsRef<Path>) -> Result<Pin<Box<Decoder>>, Box<dyn std::error::Error>> {
    let symphonia = Symphonia::new(path)?;

    let millis = 20;
    let ring_len = ((millis * symphonia.sample_rate()) / 1000) * symphonia.channels();

    let rb = HeapRb::<f32>::new(ring_len);
    let (mut prod, cons) = rb.split();

    let mut boxed = Box::pin(Decoder { symphonia, cons });

    let cast: usize = addr_of_mut!(boxed.symphonia) as usize;

    thread::spawn(move || {
        let sym = cast as *mut Symphonia;
        loop {
            if let Some(packet) = unsafe { (*sym).next_packet() } {
                for sample in packet.samples() {
                    //Wait until we can fit the packet into the buffer.
                    loop {
                        match prod.push(*sample) {
                            Ok(_) => break,
                            Err(_) => continue,
                        }
                    }
                }
            }
        }
    });

    Ok(boxed)
}

pub struct Decoder {
    pub symphonia: Symphonia,
    cons: Consumer,
}

impl Decoder {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut symphonia = Symphonia::new(path)?;
        let cast: usize = addr_of_mut!(symphonia) as usize;

        let millis = 20;
        let ring_len = ((millis * symphonia.sample_rate()) / 1000) * symphonia.channels();

        let rb = HeapRb::<f32>::new(ring_len);
        let (mut prod, cons) = rb.split();

        thread::spawn(move || {
            let sym = cast as *mut Symphonia;
            loop {
                if let Some(packet) = unsafe { (*sym).next_packet() } {
                    for sample in packet.samples() {
                        //Wait until we can fit the packet into the buffer.
                        loop {
                            match prod.push(*sample) {
                                Ok(_) => break,
                                Err(_) => continue,
                            }
                        }
                    }
                }
            }
        });
        Ok(Self { symphonia, cons })
    }
    //I hate needing to do this...
    pub fn duration(&self) -> Duration {
        self.symphonia.duration()
    }
    pub fn sample_rate(&self) -> usize {
        self.symphonia.sample_rate()
    }
}

impl Deref for Decoder {
    type Target = Consumer;

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

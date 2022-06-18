use crate::sample_rate::SampleRateConverter;
use std::{fs::File, io::ErrorKind, path::Path, time::Duration};
use symphonia::{
    core::{
        audio::{SampleBuffer, SignalSpec},
        codecs::{Decoder, DecoderOptions},
        errors::{Error, SeekErrorKind},
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::Time,
    },
    default::get_probe,
};

pub struct Generator {
    processor: Option<Processor>,
    sample_rate: u32,
    volume: f32,
}

impl Generator {
    pub fn new(sample_rate: u32, volume: f32) -> Self {
        Self {
            processor: None,
            sample_rate,
            volume,
        }
    }
    pub fn next(&mut self) -> f32 {
        if let Some(processor) = &mut self.processor {
            if processor.finished {
                0.0
            } else {
                processor.next_sample()
            }
        } else {
            0.0
        }
    }
    pub fn seek_to(&mut self, time: f32) -> Result<(), ()> {
        if let Some(processor) = &mut self.processor {
            processor.seek_to(time);
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn elapsed(&self) -> Duration {
        if let Some(processor) = &self.processor {
            processor.elapsed
        } else {
            Duration::default()
        }
    }
    pub fn duration(&self) -> Duration {
        if let Some(processor) = &self.processor {
            processor.duration
        } else {
            Duration::default()
        }
    }
    pub fn seek_by(&mut self, time: f32) -> Result<(), ()> {
        if let Some(processor) = &mut self.processor {
            processor.seek_by(time);
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        if let Some(processor) = &mut self.processor {
            processor.volume = volume;
        }
    }
    pub fn update(&mut self, path: &Path) {
        self.processor = Some(Processor::new(self.sample_rate, path, self.volume));
    }
    pub fn is_done(&self) -> bool {
        if let Some(processor) = &self.processor {
            processor.finished
        } else {
            false
        }
    }
    pub fn stop(&mut self) {
        self.processor = None;
    }
}

pub struct Processor {
    pub decoder: Box<dyn Decoder>,
    pub format: Box<dyn FormatReader>,
    pub spec: SignalSpec,
    pub capacity: u64,
    pub converter: SampleRateConverter,
    pub finished: bool,
    pub left: bool,
    pub duration: Duration,
    pub elapsed: Duration,
    pub volume: f32,
}

impl Processor {
    pub fn new(sample_rate: u32, path: &Path, volume: f32) -> Self {
        let source = Box::new(File::open(path).unwrap());

        let mss = MediaSourceStream::new(source, Default::default());

        let mut probed = get_probe()
            .format(
                &Hint::default(),
                mss,
                &FormatOptions {
                    prebuild_seek_index: true,
                    seek_index_fill_rate: 1,
                    ..Default::default()
                },
                &MetadataOptions::default(),
            )
            .unwrap();

        let track = probed.format.default_track().unwrap();

        let duration = if let Some(tb) = track.codec_params.time_base {
            let n_frames = track.codec_params.n_frames.unwrap();
            let time = tb.calc_time(n_frames);
            Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
        } else {
            panic!("Could not decode track duration.");
        };

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap();

        let current_frame = probed.format.next_packet().unwrap();
        let decoded = decoder.decode(&current_frame).unwrap();

        let spec = decoded.spec().to_owned();
        let capacity = decoded.capacity() as u64;

        let mut sample_buffer = SampleBuffer::<f32>::new(capacity, spec);
        sample_buffer.copy_interleaved_ref(decoded);

        Self {
            format: probed.format,
            decoder,
            spec,
            capacity,
            duration,
            elapsed: Duration::default(),
            converter: SampleRateConverter::new(
                sample_buffer.samples().to_vec().into_iter(),
                spec.rate,
                sample_rate,
            ),
            finished: false,
            left: true,
            volume,
        }
    }
    pub fn next_sample(&mut self) -> f32 {
        loop {
            if self.finished {
                return 0.0;
            } else if let Some(sample) = self.converter.next() {
                return sample * self.volume;
            } else {
                self.update();
            }
        }
    }
    pub fn update(&mut self) {
        if self.finished {
            return;
        }

        let mut decode_errors: usize = 0;
        const MAX_DECODE_ERRORS: usize = 3;
        loop {
            match self.format.next_packet() {
                Ok(packet) => {
                    let decoded = self.decoder.decode(&packet).unwrap();
                    let mut buffer = SampleBuffer::<f32>::new(self.capacity, self.spec);
                    buffer.copy_interleaved_ref(decoded);

                    self.converter.update(buffer.samples().to_vec().into_iter());

                    //Update elapsed
                    let ts = packet.ts();
                    let track = self.format.default_track().unwrap();
                    let tb = track.codec_params.time_base.unwrap();
                    let t = tb.calc_time(ts);
                    self.elapsed = Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);
                    return;
                }
                Err(e) => match e {
                    Error::DecodeError(e) => {
                        decode_errors += 1;
                        if decode_errors > MAX_DECODE_ERRORS {
                            panic!("{:?}", e);
                        }
                    }
                    Error::IoError(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        self.finished = true;
                        return;
                    }
                    _ => panic!("{:?}", e),
                },
            };
        }
    }
    pub fn seek_by(&mut self, time: f32) {
        let time = self.elapsed.as_secs_f32() + time;
        self.seek_to(time);
    }
    pub fn seek_to(&mut self, time: f32) {
        let time = Duration::from_secs_f32(time);
        match self.format.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time: Time::new(time.as_secs(), time.subsec_nanos() as f64 / 1_000_000_000.0),
                track_id: None,
            },
        ) {
            Ok(_) => (),
            Err(e) => match e {
                Error::SeekError(e) => match e {
                    SeekErrorKind::OutOfRange => self.finished = true,
                    _ => panic!("{:?}", e),
                },
                _ => panic!("{}", e),
            },
        }
    }
}

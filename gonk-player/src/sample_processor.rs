use crate::sample_rate::SampleRateConverter;
use std::{fs::File, path::Path, time::Duration};
use symphonia::{
    core::{
        audio::{SampleBuffer, SignalSpec},
        codecs::{Decoder, DecoderOptions},
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::Time,
    },
    default::get_probe,
};

pub struct SampleProcessor {
    pub decoder: Box<dyn Decoder>,
    pub format: Box<dyn FormatReader>,
    pub spec: SignalSpec,
    pub duration: u64,
    pub converter: SampleRateConverter,
    pub finished: bool,
    pub left: bool,
}

impl SampleProcessor {
    pub fn next_sample(&mut self) -> f32 {
        loop {
            if let Some(sample) = self.converter.next() {
                return sample * 0.1;
            } else {
                self.update();
            }
        }
    }
    pub fn update(&mut self) {
        match self.format.next_packet() {
            Ok(packet) => {
                let decoded = self.decoder.decode(&packet).unwrap();
                let mut buffer = SampleBuffer::<f32>::new(self.duration, self.spec);
                buffer.copy_interleaved_ref(decoded);

                self.converter.update(buffer.samples().to_vec().into_iter());
            }
            Err(e) => match e {
                symphonia::core::errors::Error::IoError(_) => self.finished = true,
                _ => panic!("{:?}", e),
            },
        }
    }
    pub fn seek_to(&mut self, time: Duration) {
        let nanos_per_sec = 1_000_000_000.0;
        self.format
            .seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time: Time::new(time.as_secs(), time.subsec_nanos() as f64 / nanos_per_sec),
                    track_id: None,
                },
            )
            .unwrap();
    }
    pub fn new(sample_rate: Option<u32>, path: impl AsRef<Path>) -> Self {
        let source = Box::new(File::open(path).unwrap());

        let mss = MediaSourceStream::new(source, Default::default());

        let mut probed = get_probe()
            .format(
                &Hint::default(),
                mss,
                &FormatOptions {
                    prebuild_seek_index: true,
                    ..Default::default()
                },
                &MetadataOptions::default(),
            )
            .unwrap();

        let track = probed.format.default_track().unwrap();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap();

        let current_frame = probed.format.next_packet().unwrap();
        let decoded = decoder.decode(&current_frame).unwrap();

        let spec = decoded.spec().to_owned();
        let duration = decoded.capacity() as u64;

        let mut sample_buffer = SampleBuffer::<f32>::new(duration, spec);
        sample_buffer.copy_interleaved_ref(decoded);

        Self {
            format: probed.format,
            decoder,
            spec,
            duration,
            converter: SampleRateConverter::new(
                sample_buffer.samples().to_vec().into_iter(),
                spec.rate,
                sample_rate.unwrap_or(44100),
            ),
            finished: false,
            left: true,
        }
    }
}

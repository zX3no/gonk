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
    pub capacity: u64,
    pub duration: Duration,
    pub elapsed: Duration,
    pub converter: SampleRateConverter,
    pub finished: bool,
    pub left: bool,
}

impl SampleProcessor {
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
                sample_rate.unwrap_or(44100),
            ),
            finished: false,
            left: true,
        }
    }
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
                let mut buffer = SampleBuffer::<f32>::new(self.capacity, self.spec);
                buffer.copy_interleaved_ref(decoded);

                self.converter.update(buffer.samples().to_vec().into_iter());

                //Update elapsed
                let ts = packet.ts();
                let track = self.format.default_track().unwrap();
                let tb = track.codec_params.time_base.unwrap();
                let t = tb.calc_time(ts);
                self.elapsed = Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);
            }
            Err(e) => match e {
                symphonia::core::errors::Error::IoError(_) => self.finished = true,
                _ => panic!("{:?}", e),
            },
        }
    }
    pub fn seek_by(&mut self, time: f32) {
        let time = self.elapsed.as_secs_f32() + time;
        self.seek_to(time);
    }
    pub fn seek_to(&mut self, time: f32) {
        let time = Duration::from_secs_f32(time);
        self.format
            .seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time: Time::new(time.as_secs(), time.subsec_nanos() as f64 / 1_000_000_000.0),
                    track_id: None,
                },
            )
            .unwrap();
    }
}

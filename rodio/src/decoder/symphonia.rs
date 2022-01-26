use std::{ffi::OsStr, time::Duration};
use symphonia::{
    core::{
        audio::{AudioBufferRef, SampleBuffer, SignalSpec},
        codecs::{Decoder, DecoderOptions},
        errors::Error,
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::{Time, TimeBase},
    },
    default::get_probe,
};

use crate::Source;

use super::DecoderError;

pub struct SymphoniaDecoder {
    decoder: Box<dyn Decoder>,
    current_frame_offset: usize,
    format: Box<dyn FormatReader>,
    buffer: SampleBuffer<i16>,
    spec: SignalSpec,
    total_duration: Duration,
    elapsed: Duration,
}

impl SymphoniaDecoder {
    pub fn new(mss: MediaSourceStream, extension: Option<&OsStr>) -> Result<Self, DecoderError> {
        match SymphoniaDecoder::init(mss, extension) {
            Err(e) => match e {
                Error::IoError(e) => Err(DecoderError::IoError(e.to_string())),
                Error::DecodeError(e) => Err(DecoderError::DecodeError(e)),
                Error::SeekError(_) => {
                    unreachable!("Seek errors should not occur during initialization")
                }
                Error::Unsupported(_) => Err(DecoderError::UnrecognizedFormat),
                Error::LimitError(e) => Err(DecoderError::LimitError(e)),
                Error::ResetRequired => Err(DecoderError::ResetRequired),
            },
            Ok(Some(decoder)) => Ok(decoder),
            Ok(None) => Err(DecoderError::NoStreams),
        }
    }

    pub fn into_inner(self) -> MediaSourceStream {
        self.format.into_inner()
    }

    fn init(
        mss: MediaSourceStream,
        extension: Option<&OsStr>,
    ) -> symphonia::core::errors::Result<Option<SymphoniaDecoder>> {
        let mut hint = Hint::new();
        if let Some(ext) = extension {
            hint.with_extension(ext.to_str().unwrap());
        } else {
            panic!("no hint");
        }
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let probed = get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        // let stream = match probed.format.default_track() {
        //     Some(stream) => stream,
        //     None => return Ok(None),
        // };

        // let mut decoder = symphonia::default::get_codecs()
        //     .make(&stream.codec_params, &DecoderOptions { verify: true })?;
        let mut format = probed.format;
        let decoder_opts = &DecoderOptions { verify: true };

        let track = format.default_track().unwrap();

        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, decoder_opts)?;

        let params = &track.codec_params;

        //TODO: why are there no n_frames ????
        let total_duration = if let Some(n_frames) = params.n_frames {
            if let Some(tb) = params.time_base {
                let time = tb.calc_time(n_frames);
                Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
            } else {
                panic!("no time base?");
            }
        } else {
            panic!("no n_frames");
        };

        let current_frame = format.next_packet()?;
        let decoded = decoder.decode(&current_frame)?;
        let spec = decoded.spec().to_owned();
        let buffer = SymphoniaDecoder::get_buffer(decoded, &spec);

        Ok(Some(SymphoniaDecoder {
            decoder,
            current_frame_offset: 0,
            format,
            buffer,
            spec,
            total_duration,
            elapsed: Duration::from_secs(0),
        }))
    }

    #[inline]
    fn get_buffer(decoded: AudioBufferRef, spec: &SignalSpec) -> SampleBuffer<i16> {
        let duration = decoded.capacity() as u64;
        let mut buffer = SampleBuffer::<i16>::new(duration, *spec);
        buffer.copy_interleaved_ref(decoded);
        buffer
    }
}

impl Source for SymphoniaDecoder {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.samples().len())
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.spec.channels.count() as u16
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.spec.rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }

    #[inline]
    fn elapsed(&mut self) -> Duration {
        self.elapsed
    }

    #[inline]
    fn seek(&mut self, time: Duration) -> Result<Duration, ()> {
        let nanos_per_sec = 1_000_000_000.0;
        match self.format.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time: Time::new(time.as_secs(), time.subsec_nanos() as f64 / nanos_per_sec),
                track_id: None,
            },
        ) {
            Ok(seeked_to) => {
                let base = TimeBase::new(1, self.sample_rate());
                let time = base.calc_time(seeked_to.actual_ts);

                Ok(Duration::from_millis(
                    time.seconds * 1000 + ((time.frac * 60. * 1000.).round() as u64),
                ))
            }
            Err(_) => Err(()),
        }
    }
}

impl Iterator for SymphoniaDecoder {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset == self.buffer.len() {
            match self.format.next_packet() {
                Ok(packet) => match self.decoder.decode(&packet) {
                    Ok(decoded) => {
                        self.spec = decoded.spec().to_owned();
                        self.buffer = SymphoniaDecoder::get_buffer(decoded, &self.spec);

                        let ts = packet.pts();
                        let tb = self
                            .format
                            .tracks()
                            .first()
                            .unwrap()
                            .codec_params
                            .time_base
                            .unwrap();

                        let t = tb.calc_time(ts);

                        self.elapsed =
                            Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);
                    }
                    Err(_) => return None,
                },
                Err(_) => return None,
            }
            self.current_frame_offset = 0;
        }

        let sample = self.buffer.samples()[self.current_frame_offset];
        self.current_frame_offset += 1;

        Some(sample)
    }
}

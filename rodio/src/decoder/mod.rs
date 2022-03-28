use crate::Source;
use std::{fmt, fs::File, time::Duration};
use symphonia::{
    core::{
        audio::{AudioBufferRef, SampleBuffer, SignalSpec},
        codecs,
        errors::Error,
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::{Time, TimeBase},
    },
    default::get_probe,
};

pub struct Decoder {
    decoder: Box<dyn codecs::Decoder>,
    current_frame_offset: usize,
    format: Box<dyn FormatReader>,
    buffer: SampleBuffer<i16>,
    spec: SignalSpec,
    total_duration: Duration,
    elapsed: Duration,
}

impl Decoder {
    pub fn new(file: File) -> Result<Self, DecoderError> {
        let source = Box::new(file);

        let mss = MediaSourceStream::new(source, Default::default());

        match Decoder::init(mss) {
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

    fn init(mss: MediaSourceStream) -> symphonia::core::errors::Result<Option<Decoder>> {
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let probed = get_probe().format(&Hint::new(), mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;
        let decoder_opts = &codecs::DecoderOptions { verify: true };

        let track = format.default_track().unwrap();

        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, decoder_opts)?;

        let params = &track.codec_params;

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
        let buffer = Decoder::get_buffer(decoded, &spec);

        Ok(Some(Decoder {
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

impl Source for Decoder {
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
    fn seek(&mut self, time: Duration) -> Option<Duration> {
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

                Some(Duration::from_millis(
                    time.seconds * 1000 + ((time.frac * 60. * 1000.).round() as u64),
                ))
            }
            Err(_) => None,
        }
    }
}

impl Iterator for Decoder {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset == self.buffer.len() {
            match self.format.next_packet() {
                Ok(packet) => match self.decoder.decode(&packet) {
                    Ok(decoded) => {
                        self.spec = decoded.spec().to_owned();
                        self.buffer = Decoder::get_buffer(decoded, &self.spec);

                        let ts = packet.ts();
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

/// Error that can happen when creating a decoder.
#[derive(Debug, Clone)]
pub enum DecoderError {
    /// The format of the data has not been recognized.
    UnrecognizedFormat,

    /// An IO error occured while reading, writing, or seeking the stream.
    IoError(String),

    /// The stream contained malformed data and could not be decoded or demuxed.
    DecodeError(&'static str),

    /// A default or user-defined limit was reached while decoding or demuxing the stream. Limits
    /// are used to prevent denial-of-service attacks from malicious streams.
    LimitError(&'static str),

    /// The demuxer or decoder needs to be reset before continuing.
    ResetRequired,

    /// No streams were found by the decoder
    NoStreams,
}

impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            DecoderError::UnrecognizedFormat => "Unrecognized format",
            DecoderError::IoError(msg) => &msg[..],
            DecoderError::DecodeError(msg) => msg,
            DecoderError::LimitError(msg) => msg,
            DecoderError::ResetRequired => "Reset required",
            DecoderError::NoStreams => "No streams",
        };
        write!(f, "{}", text)
    }
}
impl std::error::Error for DecoderError {}

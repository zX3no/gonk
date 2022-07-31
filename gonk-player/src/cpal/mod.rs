#![allow(unused)]

// Extern crate declarations with `#[macro_use]` must unfortunately be at crate root.
#[cfg(target_os = "emscripten")]
#[macro_use]
extern crate stdweb;

pub use error::*;
pub use platform::{
    available_hosts, default_host, host_from_id, Device, Devices, Host, HostId, Stream,
    SupportedInputConfigs, SupportedOutputConfigs, ALL_HOSTS,
};
pub use samples_formats::{Sample, SampleFormat};
use std::convert::TryInto;
use std::ops::{Div, Mul};
use std::time::Duration;

mod error;
mod host;
pub mod platform;
mod samples_formats;
pub mod traits;

pub type InputDevices<I> = std::iter::Filter<I, fn(&<I as Iterator>::Item) -> bool>;

pub type OutputDevices<I> = std::iter::Filter<I, fn(&<I as Iterator>::Item) -> bool>;

pub type ChannelCount = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(pub u32);

impl<T> Mul<T> for SampleRate
where
    u32: Mul<T, Output = u32>,
{
    type Output = Self;
    fn mul(self, rhs: T) -> Self {
        SampleRate(self.0 * rhs)
    }
}

impl<T> Div<T> for SampleRate
where
    u32: Div<T, Output = u32>,
{
    type Output = Self;
    fn div(self, rhs: T) -> Self {
        SampleRate(self.0 / rhs)
    }
}

pub type FrameCount = u32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BufferSize {
    Default,
    Fixed(FrameCount),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamConfig {
    pub channels: ChannelCount,
    pub sample_rate: SampleRate,
    pub buffer_size: BufferSize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupportedBufferSize {
    Range { min: FrameCount, max: FrameCount },

    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedStreamConfigRange {
    pub(crate) channels: ChannelCount,

    pub(crate) min_sample_rate: SampleRate,

    pub(crate) max_sample_rate: SampleRate,

    pub(crate) buffer_size: SupportedBufferSize,

    pub(crate) sample_format: SampleFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedStreamConfig {
    channels: ChannelCount,
    sample_rate: SampleRate,
    buffer_size: SupportedBufferSize,
    sample_format: SampleFormat,
}

#[derive(Debug)]
pub struct Data {
    data: *mut (),
    len: usize,
    sample_format: SampleFormat,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct StreamInstant {
    secs: i64,
    nanos: u32,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct InputStreamTimestamp {
    pub callback: StreamInstant,

    pub capture: StreamInstant,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct OutputStreamTimestamp {
    pub callback: StreamInstant,

    pub playback: StreamInstant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCallbackInfo {
    timestamp: InputStreamTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputCallbackInfo {
    timestamp: OutputStreamTimestamp,
}

impl SupportedStreamConfig {
    pub fn new(
        channels: ChannelCount,
        sample_rate: SampleRate,
        buffer_size: SupportedBufferSize,
        sample_format: SampleFormat,
    ) -> Self {
        Self {
            channels,
            sample_rate,
            buffer_size,
            sample_format,
        }
    }

    pub fn channels(&self) -> ChannelCount {
        self.channels
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn buffer_size(&self) -> &SupportedBufferSize {
        &self.buffer_size
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn config(&self) -> StreamConfig {
        StreamConfig {
            channels: self.channels,
            sample_rate: self.sample_rate,
            buffer_size: BufferSize::Default,
        }
    }
}

impl StreamInstant {
    pub fn duration_since(&self, earlier: &Self) -> Option<Duration> {
        if self < earlier {
            None
        } else {
            (self.as_nanos() - earlier.as_nanos())
                .try_into()
                .ok()
                .map(Duration::from_nanos)
        }
    }

    pub fn add(&self, duration: Duration) -> Option<Self> {
        self.as_nanos()
            .checked_add(duration.as_nanos() as i128)
            .and_then(Self::from_nanos_i128)
    }

    pub fn sub(&self, duration: Duration) -> Option<Self> {
        self.as_nanos()
            .checked_sub(duration.as_nanos() as i128)
            .and_then(Self::from_nanos_i128)
    }

    fn as_nanos(&self) -> i128 {
        (self.secs as i128 * 1_000_000_000) + self.nanos as i128
    }

    #[allow(dead_code)]
    fn from_nanos(nanos: i64) -> Self {
        let secs = nanos / 1_000_000_000;
        let subsec_nanos = nanos - secs * 1_000_000_000;
        Self::new(secs as i64, subsec_nanos as u32)
    }

    #[allow(dead_code)]
    fn from_nanos_i128(nanos: i128) -> Option<Self> {
        let secs = nanos / 1_000_000_000;
        if secs > i64::MAX as i128 || secs < i64::MIN as i128 {
            None
        } else {
            let subsec_nanos = nanos - secs * 1_000_000_000;
            debug_assert!(subsec_nanos < u32::MAX as i128);
            Some(Self::new(secs as i64, subsec_nanos as u32))
        }
    }

    #[allow(dead_code)]
    fn from_secs_f64(secs: f64) -> crate::cpal::StreamInstant {
        let s = secs.floor() as i64;
        let ns = ((secs - s as f64) * 1_000_000_000.0) as u32;
        Self::new(s, ns)
    }

    fn new(secs: i64, nanos: u32) -> Self {
        StreamInstant { secs, nanos }
    }
}

impl InputCallbackInfo {
    pub fn timestamp(&self) -> InputStreamTimestamp {
        self.timestamp
    }
}

impl OutputCallbackInfo {
    pub fn timestamp(&self) -> OutputStreamTimestamp {
        self.timestamp
    }
}

#[allow(clippy::len_without_is_empty)]
impl Data {
    // Internal constructor for host implementations to use.
    //
    // The following requirements must be met in order for the safety of `Data`'s public API.
    //
    // - The `data` pointer must point to the first sample in the slice containing all samples.
    // - The `len` must describe the length of the buffer as a number of samples in the expected
    //   format specified via the `sample_format` argument.
    // - The `sample_format` must correctly represent the underlying sample data delivered/expected
    //   by the stream.
    pub(crate) unsafe fn from_parts(
        data: *mut (),
        len: usize,
        sample_format: SampleFormat,
    ) -> Self {
        Data {
            data,
            len,
            sample_format,
        }
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn bytes(&self) -> &[u8] {
        let len = self.len * self.sample_format.sample_size();
        // The safety of this block relies on correct construction of the `Data` instance. See
        // the unsafe `from_parts` constructor for these requirements.
        unsafe { std::slice::from_raw_parts(self.data as *const u8, len) }
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        let len = self.len * self.sample_format.sample_size();
        // The safety of this block relies on correct construction of the `Data` instance. See
        // the unsafe `from_parts` constructor for these requirements.
        unsafe { std::slice::from_raw_parts_mut(self.data as *mut u8, len) }
    }

    pub fn as_slice<T>(&self) -> Option<&[T]>
    where
        T: Sample,
    {
        if T::FORMAT == self.sample_format {
            // The safety of this block relies on correct construction of the `Data` instance. See
            // the unsafe `from_parts` constructor for these requirements.
            unsafe { Some(std::slice::from_raw_parts(self.data as *const T, self.len)) }
        } else {
            None
        }
    }

    pub fn as_slice_mut<T>(&mut self) -> Option<&mut [T]>
    where
        T: Sample,
    {
        if T::FORMAT == self.sample_format {
            // The safety of this block relies on correct construction of the `Data` instance. See
            // the unsafe `from_parts` constructor for these requirements.
            unsafe {
                Some(std::slice::from_raw_parts_mut(
                    self.data as *mut T,
                    self.len,
                ))
            }
        } else {
            None
        }
    }
}

impl SupportedStreamConfigRange {
    pub fn new(
        channels: ChannelCount,
        min_sample_rate: SampleRate,
        max_sample_rate: SampleRate,
        buffer_size: SupportedBufferSize,
        sample_format: SampleFormat,
    ) -> Self {
        Self {
            channels,
            min_sample_rate,
            max_sample_rate,
            buffer_size,
            sample_format,
        }
    }

    pub fn channels(&self) -> ChannelCount {
        self.channels
    }

    pub fn min_sample_rate(&self) -> SampleRate {
        self.min_sample_rate
    }

    pub fn max_sample_rate(&self) -> SampleRate {
        self.max_sample_rate
    }

    pub fn buffer_size(&self) -> &SupportedBufferSize {
        &self.buffer_size
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn with_sample_rate(self, sample_rate: SampleRate) -> SupportedStreamConfig {
        assert!(self.min_sample_rate <= sample_rate && sample_rate <= self.max_sample_rate);
        SupportedStreamConfig {
            channels: self.channels,
            sample_rate,
            sample_format: self.sample_format,
            buffer_size: self.buffer_size,
        }
    }

    #[inline]
    pub fn with_max_sample_rate(self) -> SupportedStreamConfig {
        SupportedStreamConfig {
            channels: self.channels,
            sample_rate: self.max_sample_rate,
            sample_format: self.sample_format,
            buffer_size: self.buffer_size,
        }
    }

    pub fn cmp_default_heuristics(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::Equal;
        use SampleFormat::{F32, I16, U16};

        let cmp_stereo = (self.channels == 2).cmp(&(other.channels == 2));
        if cmp_stereo != Equal {
            return cmp_stereo;
        }

        let cmp_mono = (self.channels == 1).cmp(&(other.channels == 1));
        if cmp_mono != Equal {
            return cmp_mono;
        }

        let cmp_channels = self.channels.cmp(&other.channels);
        if cmp_channels != Equal {
            return cmp_channels;
        }

        let cmp_f32 = (self.sample_format == F32).cmp(&(other.sample_format == F32));
        if cmp_f32 != Equal {
            return cmp_f32;
        }

        let cmp_i16 = (self.sample_format == I16).cmp(&(other.sample_format == I16));
        if cmp_i16 != Equal {
            return cmp_i16;
        }

        let cmp_u16 = (self.sample_format == U16).cmp(&(other.sample_format == U16));
        if cmp_u16 != Equal {
            return cmp_u16;
        }

        const HZ_44100: SampleRate = SampleRate(44_100);
        let r44100_in_self = self.min_sample_rate <= HZ_44100 && HZ_44100 <= self.max_sample_rate;
        let r44100_in_other =
            other.min_sample_rate <= HZ_44100 && HZ_44100 <= other.max_sample_rate;
        let cmp_r44100 = r44100_in_self.cmp(&r44100_in_other);
        if cmp_r44100 != Equal {
            return cmp_r44100;
        }

        self.max_sample_rate.cmp(&other.max_sample_rate)
    }
}

impl From<SupportedStreamConfig> for StreamConfig {
    fn from(conf: SupportedStreamConfig) -> Self {
        conf.config()
    }
}

// If a backend does not provide an API for retrieving supported formats, we query it with a bunch
// of commonly used rates. This is always the case for wasapi and is sometimes the case for alsa.
//
// If a rate you desire is missing from this list, feel free to add it!
#[cfg(target_os = "windows")]
const COMMON_SAMPLE_RATES: &[SampleRate] = &[
    SampleRate(5512),
    SampleRate(8000),
    SampleRate(11025),
    SampleRate(16000),
    SampleRate(22050),
    SampleRate(32000),
    SampleRate(44100),
    SampleRate(48000),
    SampleRate(64000),
    SampleRate(88200),
    SampleRate(96000),
    SampleRate(176400),
    SampleRate(192000),
];

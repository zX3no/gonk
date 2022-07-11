use cpal::{
    traits::{HostTrait, StreamTrait},
    Stream,
};
use std::{fs::File, time::Duration, vec::IntoIter};
use symphonia::{
    core::{
        audio::SampleBuffer,
        codecs::{Decoder, DecoderOptions},
        errors::{Error, SeekErrorKind},
        formats::{FormatOptions, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::{Hint, ProbeResult},
        units::{Time, TimeBase},
    },
    default::get_probe,
};

pub use cpal::{traits::DeviceTrait, Device};
pub use index::Index;
pub use song::Song;

mod index;
mod song;

#[inline]
const fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

static mut RESAMPLER: Option<Resampler> = None;

const VOLUME_STEP: u16 = 5;
const VOLUME_REDUCTION: f32 = 600.0;

pub struct Resampler {
    probed: ProbeResult,
    decoder: Box<dyn Decoder>,

    input: usize,
    output: usize,

    buffer: IntoIter<f32>,

    current_frame: Vec<f32>,
    current_frame_pos_in_chunk: usize,

    next_frame: Vec<f32>,
    next_output_frame_pos_in_chunk: usize,

    output_buffer: Option<f32>,

    time_base: TimeBase,

    gain: f32,

    pub volume: f32,
    pub duration: Duration,
    pub finished: bool,
    pub elapsed: Duration,
}

impl Resampler {
    pub fn new(output: usize, path: &str, volume: u16, gain: f32) -> Self {
        let source = Box::new(File::open(path).unwrap());
        let mss = MediaSourceStream::new(source, Default::default());

        let mut probed = get_probe()
            .format(
                &Hint::default(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .unwrap();

        let track = probed.format.default_track().unwrap();
        let input = track.codec_params.sample_rate.unwrap() as usize;
        let time_base = track.codec_params.time_base.unwrap();

        let n_frames = track.codec_params.n_frames.unwrap();
        let time = track.codec_params.time_base.unwrap().calc_time(n_frames);
        let duration = Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac);

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap();

        let next_packet = probed.format.next_packet().unwrap();
        let decoded = decoder.decode(&next_packet).unwrap();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        let mut buffer = buffer.samples().to_vec().into_iter();

        let ts = next_packet.ts();
        let t = time_base.calc_time(ts);
        let elapsed = Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);

        let gcd = gcd(input, output);

        let (current_frame, next_frame) = if input == output {
            (Vec::new(), Vec::new())
        } else {
            (
                vec![buffer.next().unwrap(), buffer.next().unwrap()],
                vec![buffer.next().unwrap(), buffer.next().unwrap()],
            )
        };

        Self {
            probed,
            decoder,
            buffer,
            input: input / gcd,
            output: output / gcd,
            current_frame_pos_in_chunk: 0,
            next_output_frame_pos_in_chunk: 0,
            current_frame,
            next_frame,
            output_buffer: None,
            volume: volume as f32 / VOLUME_REDUCTION,
            duration,
            elapsed,
            time_base,
            finished: false,
            gain,
        }
    }

    pub fn next(&mut self) -> f32 {
        if let Some(smp) = self.next_sample() {
            if self.gain == 0.0 {
                //Reduce the volume a little to match
                //songs with replay gain information.
                smp * self.volume * 0.75
            } else {
                smp * self.volume * self.gain
            }
        } else {
            let next_packet = self.probed.format.next_packet().unwrap();
            let decoded = self.decoder.decode(&next_packet).unwrap();
            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
            buffer.copy_interleaved_ref(decoded);
            self.buffer = buffer.samples().to_vec().into_iter();

            let ts = next_packet.ts();
            let t = self.time_base.calc_time(ts);
            self.elapsed = Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);

            if self.input == self.output {
                self.current_frame = Vec::new();
                self.next_frame = Vec::new();
            } else {
                self.current_frame = vec![self.buffer.next().unwrap(), self.buffer.next().unwrap()];
                self.next_frame = vec![self.buffer.next().unwrap(), self.buffer.next().unwrap()];
            }

            self.current_frame_pos_in_chunk = 0;
            self.next_output_frame_pos_in_chunk = 0;

            debug_assert!(self.output_buffer.is_none());

            self.next()
        }
    }

    fn next_input_frame(&mut self) {
        self.current_frame = std::mem::take(&mut self.next_frame);

        if let Some(sample) = self.buffer.next() {
            self.next_frame.push(sample);
        }

        if let Some(sample) = self.buffer.next() {
            self.next_frame.push(sample);
        }

        self.current_frame_pos_in_chunk += 1;
    }

    fn next_sample(&mut self) -> Option<f32> {
        if self.input == self.output {
            return self.buffer.next();
        } else if let Some(sample) = self.output_buffer.take() {
            return Some(sample);
        }

        if self.next_output_frame_pos_in_chunk == self.output {
            self.next_output_frame_pos_in_chunk = 0;

            self.next_input_frame();
            while self.current_frame_pos_in_chunk != self.input {
                self.next_input_frame();
            }
            self.current_frame_pos_in_chunk = 0;
        } else {
            let req_left_sample =
                (self.input * self.next_output_frame_pos_in_chunk / self.output) % self.input;

            while self.current_frame_pos_in_chunk != req_left_sample {
                self.next_input_frame();
                debug_assert!(self.current_frame_pos_in_chunk < self.input);
            }
        }

        let numerator = (self.input * self.next_output_frame_pos_in_chunk) % self.output;

        self.next_output_frame_pos_in_chunk += 1;

        if self.current_frame.is_empty() && self.next_frame.is_empty() {
            return None;
        }

        if self.next_frame.is_empty() {
            let r = self.current_frame.remove(0);
            self.output_buffer = self.current_frame.first().cloned();
            self.current_frame.clear();
            Some(r)
        } else {
            let ratio = numerator as f32 / self.output as f32;
            self.output_buffer = Some(lerp(self.current_frame[1], self.next_frame[1], ratio));
            Some(lerp(self.current_frame[0], self.next_frame[0], ratio))
        }
    }

    pub fn set_volume(&mut self, volume: u16) {
        self.volume = volume as f32 / VOLUME_REDUCTION;
    }
}

#[derive(Debug, PartialEq)]
pub enum State {
    Playing,
    Paused,
    Stopped,
}

pub struct Player {
    pub stream: Stream,
    pub sample_rate: usize,
    pub state: State,
    pub songs: Index<Song>,
    pub volume: u16,
}

impl Player {
    pub fn new(_device: String, volume: u16, _cache: &[Song]) -> Self {
        let device = cpal::default_host().default_output_device().unwrap();
        let config = device.default_output_config().unwrap().config();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Some(resampler) = unsafe { &mut RESAMPLER } {
                        for frame in data.chunks_mut(2) {
                            for sample in frame.iter_mut() {
                                *sample = resampler.next();
                            }
                        }
                    }
                },
                |err| panic!("{}", err),
            )
            .unwrap();

        stream.play().unwrap();

        Self {
            sample_rate: config.sample_rate.0 as usize,
            stream,
            volume,
            state: State::Stopped,
            songs: Index::default(),
        }
    }

    pub fn update(&mut self) {
        if let Some(resampler) = unsafe { RESAMPLER.as_ref() } {
            if resampler.finished {
                self.next();
            }
        }
    }

    pub fn add_songs(&mut self, songs: &[Song]) {
        self.songs.data.extend(songs.to_vec());
        if self.songs.selected().is_none() {
            self.songs.select(Some(0));
            self.play_selected();
        }
    }

    pub fn previous(&mut self) {
        self.songs.up();
        self.play_selected();
    }

    pub fn next(&mut self) {
        self.songs.down();
        self.play_selected();
    }

    pub fn play(&mut self, path: &str) {
        unsafe {
            RESAMPLER = Some(Resampler::new(self.sample_rate, path, self.volume, 0.0));
        }
        self.state = State::Playing;
    }

    pub fn play_selected(&mut self) {
        if let Some(song) = self.songs.selected() {
            unsafe {
                RESAMPLER = Some(Resampler::new(
                    self.sample_rate,
                    song.path.to_str().unwrap(),
                    self.volume,
                    song.gain as f32,
                ));
            }
            self.state = State::Playing;
        }
    }

    pub fn play_index(&mut self, i: usize) {
        self.songs.select(Some(i));
        self.play_selected();
    }

    pub fn delete_index(&mut self, i: usize) {
        self.songs.data.remove(i);

        if let Some(playing) = self.songs.index() {
            let len = self.songs.len();

            if len == 0 {
                return self.clear();
            }

            if i == playing && i == 0 {
                if i == 0 {
                    self.songs.select(Some(0));
                }
                self.play_selected();
            } else if i == playing && i == len {
                self.songs.select(Some(len - 1));
            } else if i < playing {
                self.songs.select(Some(playing - 1));
            }
        };
    }

    pub fn clear(&mut self) {
        self.songs = Index::default();
        self.state = State::Stopped;
    }

    pub fn clear_except_playing(&mut self) {
        if let Some(index) = self.songs.index() {
            let playing = self.songs.data.remove(index);
            self.songs = Index::new(vec![playing], Some(0));
        }
    }

    pub fn volume_up(&mut self) {
        self.volume += VOLUME_STEP;
        if self.volume > 100 {
            self.volume = 100;
        }

        if let Some(resampler) = unsafe { RESAMPLER.as_mut() } {
            resampler.set_volume(self.volume);
        }
    }

    pub fn volume_down(&mut self) {
        if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        if let Some(resampler) = unsafe { RESAMPLER.as_mut() } {
            resampler.set_volume(self.volume);
        }
    }

    pub fn duration(&self) -> Duration {
        unsafe {
            match RESAMPLER.as_ref() {
                Some(resampler) => resampler.duration,
                None => Duration::default(),
            }
        }
    }

    pub fn elapsed(&self) -> Duration {
        unsafe {
            match RESAMPLER.as_ref() {
                Some(resampler) => resampler.elapsed,
                None => Duration::default(),
            }
        }
    }

    pub fn toggle_playback(&mut self) {
        match self.state {
            State::Playing => self.stream.pause().unwrap(),
            State::Paused => self.stream.play().unwrap(),
            State::Stopped => (),
        }
    }

    pub fn seek_by(&mut self, time: f32) {
        unsafe {
            if RESAMPLER.is_none() {
                return;
            }

            self.seek_to(RESAMPLER.as_ref().unwrap().elapsed.as_secs_f32() + time);
        }
    }

    pub fn seek_to(&mut self, time: f32) {
        if unsafe { RESAMPLER.is_none() } {
            return;
        }

        let time = if time.is_sign_negative() { 0.0 } else { time };

        let time = Duration::from_secs_f32(time);
        unsafe {
            match RESAMPLER.as_mut().unwrap().probed.format.seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time: Time::new(time.as_secs(), time.subsec_nanos() as f64 / 1_000_000_000.0),
                    track_id: None,
                },
            ) {
                Ok(_) => (),
                Err(e) => match e {
                    Error::SeekError(e) => match e {
                        SeekErrorKind::OutOfRange => {
                            self.next();
                        }
                        _ => panic!("{:?}", e),
                    },
                    _ => panic!("{}", e),
                },
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        State::Playing == self.state
    }
}

unsafe impl Send for Player {}

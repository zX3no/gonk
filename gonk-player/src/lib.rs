#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]
use crossbeam_channel::{bounded, Sender};
use gonk_core::{Index, Song};
use std::fs::File;
use std::io::ErrorKind;
use std::{collections::VecDeque, thread, time::Duration};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatReader, Track};
use symphonia::{
    core::{
        audio::SampleBuffer,
        codecs::{Decoder, DecoderOptions},
        formats::{FormatOptions, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::Time,
    },
    default::get_probe,
};

#[cfg(windows)]
mod wasapi;

#[cfg(windows)]
pub use wasapi::*;

#[cfg(unix)]
mod pipewire;

#[cfg(unix)]
pub use pipewire::*;

const VOLUME_REDUCTION: f32 = 150.0;

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Stopped,
    Paused,
    Playing,
    Finished,
}

#[derive(Debug)]
pub enum Event {
    /// Path, Gain
    PlaySong((String, f32)),
    /// Path, Gain, Elapsed
    RestoreSong((String, f32, f32)),
    OutputDevice(String),
    Play,
    Pause,
    Stop,
    Seek(f32),
}

pub struct Player {
    s: Sender<Event>,
    pub songs: Index<Song>,
}

impl Player {
    #[allow(clippy::new_without_default)]
    pub fn new(device: &str, volume: u8, songs: Index<Song>, elapsed: f32) -> Self {
        init();

        let devices = devices();
        let default = default_device().unwrap();
        let d = devices.iter().find(|d| d.name == device);
        let device = if let Some(d) = d { d } else { default };

        let (s, r) = bounded::<Event>(5);
        thread::spawn(move || unsafe {
            new(device, r);
        });

        //Restore previous queue state.
        unsafe { VOLUME = volume as f32 / VOLUME_REDUCTION };
        if let Some(song) = songs.selected().cloned() {
            s.send(Event::RestoreSong((song.path.clone(), song.gain, elapsed)))
                .unwrap();
        }

        Self { s, songs }
    }
    pub fn play(&self) {
        self.s.send(Event::Play).unwrap();
    }
    pub fn pause(&self) {
        self.s.send(Event::Pause).unwrap();
    }
    pub fn seek(&self, pos: f32) {
        self.s.send(Event::Seek(pos)).unwrap();
    }
    pub fn volume_up(&self) {
        unsafe {
            VOLUME =
                ((VOLUME * VOLUME_REDUCTION) as u8 + 5).clamp(0, 100) as f32 / VOLUME_REDUCTION;
        }
    }
    pub fn volume_down(&self) {
        unsafe {
            VOLUME =
                ((VOLUME * VOLUME_REDUCTION) as i8 - 5).clamp(0, 100) as f32 / VOLUME_REDUCTION;
        }
    }
    pub fn elapsed(&self) -> Duration {
        unsafe { ELAPSED }
    }
    pub fn duration(&self) -> Duration {
        unsafe { DURATION }
    }
    pub fn is_playing(&self) -> bool {
        unsafe { STATE == State::Playing }
    }
    pub fn next(&mut self) {
        self.songs.down();
        if let Some(song) = self.songs.selected() {
            unsafe { STATE == State::Playing };
            self.s
                .send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
        }
    }
    pub fn prev(&mut self) {
        self.songs.up();
        if let Some(song) = self.songs.selected() {
            self.s
                .send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
        }
    }
    pub fn delete_index(&mut self, index: usize) {
        if self.songs.is_empty() {
            return;
        }

        self.songs.remove(index);

        if let Some(playing) = self.songs.index() {
            let len = self.songs.len();
            if len == 0 {
                self.clear();
            } else if index == playing && index == 0 {
                self.songs.select(Some(0));
                self.play_index(self.songs.index().unwrap());
            } else if index == playing && index == len {
                self.songs.select(Some(len - 1));
                self.play_index(self.songs.index().unwrap());
            } else if index < playing {
                self.songs.select(Some(playing - 1));
            }
        };
    }
    pub fn clear(&mut self) {
        self.s.send(Event::Stop).unwrap();
        self.songs = Index::default();
    }
    pub fn clear_except_playing(&mut self) {
        if let Some(index) = self.songs.index() {
            let playing = self.songs.remove(index);
            self.songs = Index::new(vec![playing], Some(0));
        }
    }
    pub fn add(&mut self, songs: Vec<Song>) {
        self.songs.extend(songs);
        if self.songs.selected().is_none() {
            self.songs.select(Some(0));
            self.play_index(0);
        }
    }
    pub fn play_index(&mut self, i: usize) {
        self.songs.select(Some(i));
        if let Some(song) = self.songs.selected() {
            self.s
                .send(Event::PlaySong((song.path.clone(), song.gain)))
                .unwrap();
        }
    }
    pub fn toggle_playback(&self) {
        match unsafe { &STATE } {
            State::Paused => self.play(),
            State::Playing => self.pause(),
            _ => (),
        }
    }
    pub fn is_finished(&self) -> bool {
        unsafe { STATE == State::Finished }
    }
    pub fn seek_foward(&mut self) {
        let pos = (self.elapsed().as_secs_f32() + 10.0).clamp(0.0, f32::MAX);
        self.seek(pos);
    }
    pub fn seek_backward(&mut self) {
        let pos = (self.elapsed().as_secs_f32() - 10.0).clamp(0.0, f32::MAX);
        self.seek(pos);
    }
    pub fn volume(&self) -> u8 {
        unsafe { (VOLUME * VOLUME_REDUCTION) as u8 }
    }
    pub fn set_output_device(&self, device: &str) {
        self.s
            .send(Event::OutputDevice(device.to_string()))
            .unwrap();
    }
}

pub struct Symphonia {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: Track,
    elapsed: u64,
    duration: u64,
    error_count: u8,
    buf: VecDeque<f32>,
}

impl Symphonia {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
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
            .make(&track.codec_params, &DecoderOptions::default())?;

        Ok(Self {
            format_reader: probed.format,
            decoder,
            track,
            duration,
            elapsed: 0,
            error_count: 0,
            buf: VecDeque::new(),
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
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<f32> {
        if self.buf.is_empty() {
            match self.next_packet() {
                Some(packet) => self.buf = VecDeque::from(packet.samples().to_vec()),
                None => {
                    return None;
                }
            }
        }

        self.buf.pop_front()
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

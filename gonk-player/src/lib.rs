use cpal::{
    traits::{HostTrait, StreamTrait},
    StreamError,
};
use crossbeam_channel::{unbounded, Sender};
use sample_processor::Generator;
use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

mod index;
mod sample_processor;
mod sample_rate;
mod song;

pub use cpal::traits::DeviceTrait;
pub use cpal::Device;
pub use index::Index;
pub use song::Song;

const VOLUME_STEP: u16 = 5;
const VOLUME_REDUCTION: f32 = 600.0;

#[derive(Debug)]
pub enum Event {
    Play,
    Pause,
    SeekBy(f32),
    SeekTo(f32),
    Volume(f32),
}

pub struct Player {
    s: Sender<Event>,
    playing: bool,
    volume: u16,
    songs: Index<Song>,
    elapsed: Arc<RwLock<Duration>>,
    generator: Arc<RwLock<Generator>>,
    duration: Duration,
}

impl Player {
    pub fn new(volume: u16) -> Self {
        let (s, r) = unbounded();

        let device = cpal::default_host().default_output_device().unwrap();
        let config = device.default_output_config().unwrap();
        let rate = config.sample_rate().0;

        let generator = Arc::new(RwLock::new(Generator::new(
            rate,
            volume as f32 / VOLUME_REDUCTION,
        )));
        let gen = generator.clone();
        let g = generator.clone();

        let elapsed = Arc::new(RwLock::new(Duration::default()));
        let e = elapsed.clone();

        thread::spawn(move || {
            let stream = device
                .build_output_stream(
                    &config.config(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        for frame in data.chunks_mut(2) {
                            for sample in frame.iter_mut() {
                                *sample = g.write().unwrap().next();
                            }
                        }
                    },
                    |err| panic!("{}", err),
                )
                .unwrap();

            stream.play().unwrap();

            loop {
                *e.write().unwrap() = gen.read().unwrap().elapsed();

                if let Ok(event) = r.recv_timeout(Duration::from_millis(8)) {
                    match event {
                        Event::Play => stream.play().unwrap(),
                        Event::Pause => stream.pause().unwrap(),
                        Event::SeekBy(duration) => gen.write().unwrap().seek_by(duration).unwrap(),
                        Event::SeekTo(duration) => gen.write().unwrap().seek_to(duration).unwrap(),
                        Event::Volume(volume) => gen.write().unwrap().set_volume(volume),
                    }
                }
            }
        });

        Self {
            s,
            playing: false,
            volume,
            elapsed,
            duration: Duration::default(),
            songs: Index::default(),
            generator,
        }
    }
    pub fn update(&mut self) {
        if self.generator.read().unwrap().is_done() {
            self.next();
        }
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn elapsed(&self) -> Duration {
        *self.elapsed.read().unwrap()
    }
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }
    pub fn add_songs(&mut self, songs: &[Song]) {
        self.songs.data.extend(songs.to_vec());
        if self.songs.selected().is_none() {
            self.songs.select(Some(0));
            self.play_selected();
        }
    }
    pub fn get_volume(&self) -> u16 {
        self.volume
    }
    pub fn play_selected(&mut self) {
        if let Some(song) = self.songs.selected() {
            self.playing = true;
            let mut gen = self.generator.write().unwrap();
            gen.update(&song.path.clone());
            gen.set_volume(self.real_volume());
            self.duration = gen.duration();
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
        self.generator.write().unwrap().stop();
    }
    pub fn clear_except_playing(&mut self) {
        let selected = self.songs.selected().cloned();
        let mut i = 0;
        while i < self.songs.len() {
            if Some(&self.songs.data[i]) != selected.as_ref() {
                self.songs.data.remove(i);
            } else {
                i += 1;
            }
        }
        self.songs.select(Some(0));
    }
    pub fn toggle_playback(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
        }
    }
    fn play(&mut self) {
        self.s.send(Event::Play).unwrap();
        self.playing = true;
    }
    fn pause(&mut self) {
        self.s.send(Event::Pause).unwrap();
        self.playing = false;
    }
    pub fn previous(&mut self) {
        self.songs.up();
        self.play_selected();
    }
    pub fn next(&mut self) {
        self.songs.down();
        self.play_selected();
    }
    pub fn volume_up(&mut self) {
        self.volume += VOLUME_STEP;

        if self.volume > 100 {
            self.volume = 100;
        }

        self.update_volume();
    }
    pub fn volume_down(&mut self) {
        if self.volume != 0 {
            self.volume -= VOLUME_STEP;
        }

        self.update_volume();
    }
    pub fn randomize(&self) {}
    fn update_volume(&self) {
        self.s.send(Event::Volume(self.real_volume())).unwrap();
    }
    fn real_volume(&self) -> f32 {
        if let Some(song) = self.songs.selected() {
            let volume = self.volume as f32 / VOLUME_REDUCTION;
            //Calculate the volume with gain
            if song.track_gain == 0.0 {
                //Reduce the volume a little to match
                //songs with replay gain information.
                volume * 0.75
            } else {
                volume * song.track_gain as f32
            }
        } else {
            self.volume as f32 / VOLUME_REDUCTION
        }
    }
    pub fn is_playing(&self) -> bool {
        self.playing
    }
    pub fn total_songs(&self) -> usize {
        self.songs.len()
    }
    pub fn get_index(&self) -> &Index<Song> {
        &self.songs
    }

    pub fn selected_song(&self) -> Option<&Song> {
        self.songs.selected()
    }
    pub fn seek_by(&self, duration: f32) {
        self.s.send(Event::SeekBy(duration)).unwrap();
    }
    pub fn seek_to(&self, duration: f32) {
        self.s.send(Event::SeekTo(duration)).unwrap();
    }
    pub fn audio_devices() -> Vec<Device> {
        let host_id = cpal::default_host().id();
        let host = cpal::host_from_id(host_id).unwrap();

        //FIXME: Getting just the output devies was too slow(150ms).
        //Collecting every device is still slow but it's not as bad.
        host.devices().unwrap().collect()
    }
    pub fn default_device() -> Device {
        cpal::default_host().default_output_device().unwrap()
    }
    pub fn change_output_device(&mut self, _device: &Device) -> Result<(), StreamError> {
        //TODO
        Ok(())
        // match OutputStream::try_from_device(device) {
        //     Ok((stream, handle)) => {
        //         let pos = self.elapsed();
        //         self.stop();
        //         self.stream = stream;
        //         self.handle = handle;
        //         self.play_selected();
        //         self.seek_to(pos);
        //         Ok(())
        //     }
        //     Err(e) => match e {
        //         stream::StreamError::DefaultStreamConfigError(_) => Ok(()),
        //         _ => Err(e),
        //     },
        // }
    }
}

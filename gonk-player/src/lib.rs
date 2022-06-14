use cpal::{
    traits::{HostTrait, StreamTrait},
    StreamError,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use sample_processor::SampleProcessor;
use std::{
    path::PathBuf,
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

#[derive(Debug)]
pub enum Event {
    Play,
    Pause,
    Stop,
    SeekBy(f32),
    SeekTo(f32),
}

pub struct Player {
    s: Sender<Event>,
    r: Receiver<Event>,
    playing: bool,
    volume: u16,
    songs: Index<Song>,
    duration: Arc<RwLock<Duration>>,
    elapsed: Arc<RwLock<Duration>>,
}

impl Player {
    pub fn new(volume: u16) -> Self {
        let (s, r) = unbounded();
        Self {
            s,
            r,
            playing: false,
            volume,
            duration: Arc::new(RwLock::new(Duration::default())),
            elapsed: Arc::new(RwLock::new(Duration::default())),
            songs: Index::default(),
        }
    }
    pub fn update(&mut self) {
        // if self.elapsed() > self.duration {
        //     self.next_song();
        // }
    }
    pub fn duration(&self) -> Duration {
        *self.duration.read().unwrap()
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
            if self.playing {
                self.stop();
            }
            self.playing = true;
            self.run(song.path.clone());
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
        self.stop();
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
    pub fn randomize(&self) {}
    pub fn toggle_playback(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
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
    pub fn volume_up(&self) {}
    pub fn volume_down(&self) {}
    pub fn is_playing(&self) -> bool {
        self.playing
    }
    pub fn total_songs(&self) -> usize {
        self.songs.len()
    }
    fn play(&mut self) {
        self.s.send(Event::Play).unwrap();
        self.playing = true;
    }

    pub fn get_index(&self) -> &Index<Song> {
        &self.songs
    }
    fn pause(&mut self) {
        self.s.send(Event::Pause).unwrap();
        self.playing = false;
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
    pub fn stop(&self) {
        self.s.send(Event::Stop).unwrap();
    }
    fn run(&self, path: PathBuf) {
        let r = self.r.clone();
        let duration = self.duration.clone();
        let elapsed = self.elapsed.clone();

        thread::spawn(move || {
            let device = cpal::default_host().default_output_device().unwrap();
            let config = device.default_output_config().unwrap();

            let processor = Arc::new(RwLock::new(SampleProcessor::new(
                Some(config.sample_rate().0),
                path,
            )));

            //Update the duration;
            *duration.write().unwrap() = processor.read().unwrap().duration;

            let p = processor.clone();

            let stream = device
                .build_output_stream(
                    &config.config(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        for frame in data.chunks_mut(2) {
                            for sample in frame.iter_mut() {
                                *sample = p.write().unwrap().next_sample();
                            }
                        }
                    },
                    |err| panic!("{}", err),
                )
                .unwrap();

            stream.play().unwrap();

            loop {
                *elapsed.write().unwrap() = processor.read().unwrap().elapsed;

                if let Ok(event) = r.recv_timeout(Duration::from_millis(16)) {
                    match event {
                        Event::Play => stream.play().unwrap(),
                        Event::Pause => stream.pause().unwrap(),
                        Event::SeekBy(duration) => processor.write().unwrap().seek_by(duration),
                        Event::SeekTo(duration) => processor.write().unwrap().seek_to(duration),
                        Event::Stop => break,
                    }
                }
            }
        });
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

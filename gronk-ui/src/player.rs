use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, RwLock},
    thread,
};

use gronk_types::Song;
use rodio::*;

pub struct Player {
    sink: Arc<RwLock<Option<Sink>>>,
    volume: f32,
}
impl Player {
    pub fn new() -> Self {
        Self {
            sink: Arc::new(RwLock::new(None)),
            volume: 0.01,
        }
    }
    pub fn play(&mut self, song: &Song) {
        let volume = self.volume.clone();
        let path = song.path.clone();
        let s = self.sink.clone();
        thread::spawn(move || {
            let (_stream, handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&handle).unwrap();
            let file = File::open(path).unwrap();
            sink.set_volume(volume);
            sink.append(Decoder::new(BufReader::new(file)).unwrap());
            *s.write().unwrap() = Some(sink);
            s.read().unwrap().as_ref().unwrap().sleep_until_end();
        });
    }
    pub fn volume(&mut self, vol: f32) {
        self.volume += vol;

        if self.volume > 1.0 {
            self.volume = 1.0;
        } else if self.volume < 0.0 {
            self.volume = 0.0;
        }

        if let Some(sink) = &*self.sink.read().unwrap() {
            sink.set_volume(self.volume);
        }
    }
    pub fn toggle_playback(&self) {
        if let Some(sink) = &*self.sink.read().unwrap() {
            if sink.is_paused() {
                sink.play()
            } else {
                sink.pause();
            }
        }
    }
    pub fn stop(&self) {
        if let Some(sink) = &*self.sink.read().unwrap() {
            sink.stop();
        }
    }
}

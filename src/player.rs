use rodio::Sink;
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

pub struct Command {
    playing: bool,
}
impl Command {
    pub fn new() -> Self {
        Self { playing: false }
    }
}
#[derive(Clone)]
pub enum Event {
    Play,
    Pause,
    Stop,
    Volume(f32),
    Empty,
}

pub struct Player {
    pub now_playing: String,
    playing: bool,
    volume: f32,
    pub event: Arc<RwLock<Event>>,
}
impl Player {
    pub fn new() -> Self {
        Self {
            now_playing: String::new(),
            playing: false,
            volume: 0.01,
            event: Arc::new(RwLock::new(Event::Empty)),
        }
    }
    pub fn play(&mut self, path: &PathBuf) {
        //kill any other song
        self.stop();
        thread::sleep(Duration::from_millis(25));

        self.now_playing = path.file_name().unwrap().to_string_lossy().to_string();
        self.playing = true;

        let path = path.clone();

        //reset the event
        *self.event.write().unwrap() = Event::Empty;
        let event = self.event.clone();
        let volume = self.volume.clone();

        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let file = BufReader::new(File::open(path).unwrap());
            let source = Decoder::new(file).unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();

            sink.append(source);
            sink.set_volume(volume);
            loop {
                match *event.read().unwrap() {
                    Event::Play => sink.play(),
                    Event::Pause => sink.pause(),
                    Event::Stop => sink.stop(),
                    Event::Volume(v) => sink.set_volume(v),
                    Event::Empty => (),
                }
                thread::sleep(Duration::from_millis(16));
            }
        });
    }
    pub fn toggle_playback(&mut self) {
        if self.playing {
            *self.event.write().unwrap() = Event::Pause;
        } else {
            *self.event.write().unwrap() = Event::Play;
        }
        self.playing = !self.playing;
    }
    pub fn stop(&mut self) {
        *self.event.write().unwrap() = Event::Stop;
        self.playing = false;
    }
    pub fn increase_volume(&mut self) {
        self.volume += 0.005;
        *self.event.write().unwrap() = Event::Volume(self.volume);
    }
    pub fn decrease_volume(&mut self) {
        self.volume -= 0.005;
        *self.event.write().unwrap() = Event::Volume(self.volume);
    }
}

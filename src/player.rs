use rodio::Sink;
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub struct Player {}
impl Player {
    pub fn play(path: &PathBuf) {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        // Load a sound from a file, using a path relative to Cargo.toml
        let file = BufReader::new(File::open(path).unwrap());
        // Decode that sound file into a source
        let source = Decoder::new(file).unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        sink.append(source);
        sink.set_volume(0.01);
        sink.sleep_until_end();
        // sink.play();
        // thread::sleep(Duration::from_secs(20));
    }
}

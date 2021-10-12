use soloud::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct Player {
    now_playing: String,
    sl: Arc<RwLock<Soloud>>,
    handle: Arc<RwLock<Option<Handle>>>,
    thread_handle: Option<JoinHandle<()>>,
    song_length: Arc<RwLock<f64>>,
    elapsed: Arc<RwLock<f64>>,
    quit: Arc<AtomicBool>,

    pub volume: f32,
}
impl Player {
    pub fn new() -> Self {
        //wtf?
        Self {
            sl: Arc::new(RwLock::new(Soloud::default().unwrap())),
            handle: Arc::new(RwLock::new(None)),
            thread_handle: None,
            song_length: Arc::new(RwLock::new(0.0)),
            elapsed: Arc::new(RwLock::new(0.0)),
            now_playing: String::new(),
            volume: 0.01,
            quit: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn play(&mut self, path: &Path) {
        if self.thread_handle.is_some() {
            //stop playback smoothly
            self.stop();
            //tell the thread to quit
            self.quit.store(true, Ordering::SeqCst);
            //wait for thread to quit
            self.thread_handle.take().unwrap().join().unwrap();
            //keep new thread alive
            self.quit.store(false, Ordering::SeqCst);
        }

        self.now_playing = path.file_stem().unwrap().to_string_lossy().to_string();
        let path = path.to_path_buf();
        let handle = self.handle.clone();
        let sl = self.sl.clone();
        let length = self.song_length.clone();
        let elapsed = self.elapsed.clone();
        let quit = self.quit.clone();

        self.thread_handle = Some(thread::spawn(move || {
            let mut wav = audio::Wav::default();
            wav.load(path).unwrap();
            *length.write().unwrap() = wav.length();
            *handle.write().unwrap() = Some(sl.read().unwrap().play(&wav));

            sl.write()
                .unwrap()
                .set_volume(handle.read().unwrap().unwrap(), 0.02);

            while !quit.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(25));
                *elapsed.write().unwrap() = sl
                    .read()
                    .unwrap()
                    .stream_position(handle.read().unwrap().unwrap());
            }
        }));
    }
    pub fn now_playing(&self) -> String {
        self.now_playing.clone()
    }

    pub fn progress(&self) -> String {
        format!("{}/{}", self.get_elapsed(), self.get_length())
    }
    pub fn progress_percent(&self) -> u16 {
        let len = *self.song_length.read().unwrap();
        let el = *self.elapsed.read().unwrap();

        let percent = (el / len * 100.0) as u16;
        percent.clamp(0, 100)
    }
    fn get_length(&self) -> String {
        let secs = *self.song_length.read().unwrap();

        let mins = secs / 60.0;
        let rem = secs % 60.0;
        format!(
            "{:0width$}:{:0width$}",
            mins.trunc() as usize,
            rem.trunc() as usize,
            width = 2,
        )
    }
    fn get_elapsed(&self) -> String {
        let e = *self.elapsed.read().unwrap();
        let mins = e / 60.0;
        let rem = e % 60.0;
        format!(
            "{:0width$}:{:0width$}",
            mins.trunc() as usize,
            rem.trunc() as usize,
            width = 2,
        )
    }
    pub fn toggle_playback(&mut self) {
        let paused = self
            .sl
            .read()
            .unwrap()
            .pause(self.handle.read().unwrap().unwrap());

        self.sl
            .write()
            .unwrap()
            .set_pause(self.handle.read().unwrap().unwrap(), !paused)
    }
    pub fn stop(&mut self) {
        self.sl.write().unwrap().stop_all();
    }
    pub fn increase_volume(&mut self) {
        self.volume += 0.002;
        if self.volume > 0.05 {
            self.volume = 0.05;
        }

        self.sl
            .write()
            .unwrap()
            .set_volume(self.handle.read().unwrap().unwrap(), self.volume);
    }
    pub fn decrease_volume(&mut self) {
        self.volume -= 0.002;
        if self.volume < 0.0 {
            self.volume = 0.0;
        }
        self.sl
            .write()
            .unwrap()
            .set_volume(self.handle.read().unwrap().unwrap(), self.volume);
    }
}

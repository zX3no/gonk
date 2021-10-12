use crossbeam_channel::{tick, unbounded, Receiver, Sender};
use soloud::*;
use std::fs::File;
use std::io::BufReader;
use std::os::windows::prelude::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{mem, thread};
use winapi::um::processthreadsapi::TerminateThread;

pub struct Player {
    pub now_playing: String,
    sl: Arc<RwLock<Soloud>>,
    handle: Arc<RwLock<Option<Handle>>>,
    thread_handle: Option<JoinHandle<()>>,
    song_length: Arc<RwLock<f64>>,
    elapsed: Arc<RwLock<f64>>,

    pub volume: f32,
}
impl Player {
    pub fn new() -> Self {
        Self {
            sl: Arc::new(RwLock::new(Soloud::default().unwrap())),
            handle: Arc::new(RwLock::new(None)),
            thread_handle: None,
            song_length: Arc::new(RwLock::new(0.0)),
            elapsed: Arc::new(RwLock::new(0.0)),
            now_playing: String::new(),
            volume: 0.01,
        }
    }
    pub fn play(&mut self, path: &PathBuf) {
        //stop the music and kill the thread
        self.sl.write().unwrap().stop_all();
        self.kill_thread();

        let path = path.clone();

        let handle = self.handle.clone();
        let sl = self.sl.clone();
        let length = self.song_length.clone();
        let elapsed = self.elapsed.clone();

        self.thread_handle = Some(thread::spawn(move || {
            let mut wav = audio::Wav::default();
            wav.load(path).unwrap();
            *length.write().unwrap() = wav.length();
            *handle.write().unwrap() = Some(sl.read().unwrap().play(&wav));

            sl.write()
                .unwrap()
                .set_volume(handle.read().unwrap().unwrap(), 0.02);

            //I sleep
            loop {
                thread::sleep(Duration::from_millis(25));
                *elapsed.write().unwrap() = sl
                    .read()
                    .unwrap()
                    .stream_position(handle.read().unwrap().unwrap());
            }
            // thread::park();
        }));
    }
    //this causes a memory leak
    pub fn kill_thread(&mut self) {
        if let Some(handle) = &self.thread_handle {
            unsafe {
                TerminateThread(handle.as_raw_handle(), 1);
            }
        }
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
        // let a = Duration::from_secs(secs as u64);
        // if secs > 0.0 {
        //     panic!("{}, {}", mins.trunc(), rem.trunc());
        // }
        // return a;
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

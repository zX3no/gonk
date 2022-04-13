use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use crate::stream::{OutputStreamHandle, PlayError};
use crate::{queue, source::Done, Sample, Source};

/// Handle to an device that outputs sounds.
///
/// Dropping the `Sink` stops all sounds. You can use `detach` if you want the sounds to continue
/// playing.
pub struct Sink {
    queue_tx: Arc<queue::SourcesQueueInput<f32>>,
    sleep_until_end: Mutex<Option<Receiver<()>>>,

    controls: Arc<Controls>,
    sound_count: Arc<AtomicUsize>,

    detached: bool,

    elapsed: Arc<RwLock<Duration>>,
}

struct Controls {
    pause: AtomicBool,
    volume: Mutex<f32>,
    seek: Mutex<Option<Duration>>,
    stopped: AtomicBool,
}

impl Sink {
    /// Builds a new `Sink`, beginning playback on a stream.
    #[inline]
    pub fn try_new(stream: &OutputStreamHandle) -> Result<Sink, PlayError> {
        optick::event!("try_new sink");
        let (sink, queue_rx) = Sink::new_idle();
        stream.play_raw(queue_rx)?;
        Ok(sink)
    }

    /// Builds a new `Sink`.
    #[inline]
    pub fn new_idle() -> (Sink, queue::SourcesQueueOutput<f32>) {
        let (queue_tx, queue_rx) = queue::queue(true);

        let sink = Sink {
            queue_tx,
            sleep_until_end: Mutex::new(None),
            controls: Arc::new(Controls {
                pause: AtomicBool::new(false),
                volume: Mutex::new(1.0),
                stopped: AtomicBool::new(false),
                seek: Mutex::new(None),
            }),
            sound_count: Arc::new(AtomicUsize::new(0)),
            detached: false,
            elapsed: Arc::new(RwLock::new(Duration::from_secs(0))),
        };
        (sink, queue_rx)
    }

    /// Appends a sound to the queue of sounds to play.
    #[inline]
    pub fn append<S>(&self, source: S)
    where
        S: Source + Send + 'static,
        S::Item: Sample,
        S::Item: Send,
    {
        let controls = self.controls.clone();

        let elapsed = self.elapsed.clone();
        let source = source
            .pausable(false)
            .amplify(1.0)
            .stoppable()
            .periodic_access(Duration::from_millis(5), move |src| {
                if controls.stopped.load(Ordering::SeqCst) {
                    src.stop();
                } else {
                    if let Some(seek_time) = controls.seek.lock().unwrap().take() {
                        src.seek(seek_time).unwrap();
                    }
                    *elapsed.write().unwrap() = src.elapsed();
                    src.inner_mut().set_factor(*controls.volume.lock().unwrap());
                    src.inner_mut()
                        .inner_mut()
                        .set_paused(controls.pause.load(Ordering::SeqCst));
                }
            })
            .convert_samples();
        self.sound_count.fetch_add(1, Ordering::Relaxed);
        let source = Done::new(source, self.sound_count.clone());
        *self.sleep_until_end.lock().unwrap() = Some(self.queue_tx.append_with_signal(source));
    }

    /// Gets the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than 1.0 will
    /// multiply each sample by this value.
    #[inline]
    pub fn volume(&self) -> f32 {
        *self.controls.volume.lock().unwrap()
    }

    /// Changes the volume of the sound.
    ///
    /// The value `1.0` is the "normal" volume (unfiltered input). Any value other than `1.0` will
    /// multiply each sample by this value.
    #[inline]
    pub fn set_volume(&self, value: f32) {
        *self.controls.volume.lock().unwrap() = value;
    }

    /// Resumes playback of a paused sink.
    ///
    /// No effect if not paused.
    #[inline]
    pub fn play(&self) {
        self.controls.pause.store(false, Ordering::SeqCst);
    }

    /// Pauses playback of this sink.
    ///
    /// No effect if already paused.
    ///
    /// A paused sink can be resumed with `play()`.
    pub fn pause(&self) {
        self.controls.pause.store(true, Ordering::SeqCst);
    }

    /// Toggles playback of the sink
    pub fn toggle_playback(&self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    pub fn seek(&self, seek_time: Duration) {
        *self.controls.seek.lock().unwrap() = Some(seek_time);
    }

    /// Gets if a sink is paused
    ///
    /// Sinks can be paused and resumed using `pause()` and `play()`. This returns `true` if the
    /// sink is paused.
    pub fn is_paused(&self) -> bool {
        self.controls.pause.load(Ordering::SeqCst)
    }

    /// Destroys the sink without stopping the sounds that are still playing.
    #[inline]
    pub fn detach(mut self) {
        self.detached = true;
    }

    /// Sleeps the current thread until the sound ends.
    #[inline]
    pub fn sleep_until_end(&self) {
        if let Some(sleep_until_end) = self.sleep_until_end.lock().unwrap().take() {
            let _ = sleep_until_end.recv();
        }
    }
    /// Returns true if this sink has no more sounds to play.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of sounds currently in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.sound_count.load(Ordering::Relaxed)
    }

    #[inline]
    //TODO: change this to f64
    pub fn elapsed(&self) -> Duration {
        *self.elapsed.read().unwrap()
    }
}

impl Drop for Sink {
    #[inline]
    fn drop(&mut self) {
        self.queue_tx.set_keep_alive_if_empty(false);

        if !self.detached {
            self.controls.stopped.store(true, Ordering::Relaxed);
        }
    }
}

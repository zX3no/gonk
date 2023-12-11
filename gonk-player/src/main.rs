#![feature(const_float_bits_conv)]
use gonk_player::{decoder::Symphonia, *};
use makepad_windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
use mini::*;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, AtomicU8, Ordering},
        Condvar, Mutex,
    },
    thread,
    time::Duration,
};

pub struct AtomicF32 {
    storage: AtomicU32,
}

impl AtomicF32 {
    pub const fn new(value: f32) -> Self {
        let as_u64 = value.to_bits();
        Self {
            storage: AtomicU32::new(as_u64),
        }
    }
    pub fn store(&self, value: f32, ordering: Ordering) {
        let as_u32 = value.to_bits();
        self.storage.store(as_u32, ordering)
    }
    pub fn load(&self, ordering: Ordering) -> f32 {
        let as_u32 = self.storage.load(ordering);
        f32::from_bits(as_u32)
    }
}

//Two threads should always be running
//The wasapi thread and the decoder thread.
//The wasapi thread reads from a buffer, if the buffer is empty, block until it's not.
//The decoder thread needs a way to request a new file to be read.
//It should read the contents of the audio file into the shared buffer.

static mut BUFFER: Option<boxcar::Vec<f32>> = None;

static mut PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static mut PATH_CONDVAR: Condvar = Condvar::new();

static mut ELAPSED: Duration = Duration::from_secs(0);
static mut DURATION: Duration = Duration::from_secs(0);
static mut SEEK: Option<f32> = None;
static mut VOLUME: AtomicF32 = AtomicF32::new(0.05);

static mut COMMAND: AtomicU8 = AtomicU8::new(NONE);

const NONE: u8 = 0;
const STOP: u8 = 1;
const PAUSE: u8 = 2;

pub unsafe fn decoder_thread() {
    BUFFER = Some(boxcar::Vec::new());
    thread::spawn(|| {
        info!("Spawned decoder thread!");
        // let mut lock = PATH_CONDVAR.wait(PATH.lock().unwrap()).unwrap();
        // let mut path = lock.as_ref().unwrap();
        // info!("Decoder thread unlocked!");

        let mut lock;
        let mut path: Option<&PathBuf> = None;

        loop {
            if let Some(path) = path {
                info!("Playing file: {}", path.display());
                let mut sym = Symphonia::new(path).unwrap();
                DURATION = sym.duration();

                //TODO: We need to cache every packet length and the timestamp.
                //This way we can calculate the current time but getting the buffer index.
                //Seeking will need to be compeletely redesigned.
                //When seeking with symphonia the mediasourcestream will be updated.
                //This would break my buffer. If the packet is already loaded I want to use it.
                while let Some(packet) = sym.next_packet() {
                    BUFFER.as_mut().unwrap().extend(packet.samples().to_vec());

                    if let Some(seek) = SEEK {
                        info!(
                            "Seeking {} / {}",
                            seek as u32,
                            DURATION.as_secs_f32() as u32
                        );
                        BUFFER = Some(boxcar::Vec::new());
                        sym.seek(seek);
                        SEEK = None;
                    }

                    match COMMAND.load(Ordering::Relaxed) {
                        NONE => {}
                        STOP => {
                            break;
                        }
                        PAUSE => {
                            while COMMAND.load(Ordering::Relaxed) == PAUSE {
                                std::hint::spin_loop();
                            }
                        }
                        _ => {}
                    }

                    ELAPSED = sym.elapsed();
                }
            }

            lock = PATH.lock().unwrap();

            if let Some(p) = lock.as_ref() {
                path = Some(p);
            } else {
                info!("Waiting for a new file.");
                lock = PATH_CONDVAR.wait(lock).unwrap();
                path = Some(lock.as_ref().unwrap());
            }
        }
    });
}

pub unsafe fn wasapi_thread() {
    thread::spawn(|| {
        info!("Spawned WASAPI thread!");
        let default = default_device();
        let wasapi = Wasapi::new(&default, Some(44100)).unwrap();
        let block_align = wasapi.format.Format.nBlockAlign as u32;
        let mut i = 0;

        loop {
            std::hint::spin_loop();
            //Sample-rate probably changed if this fails.
            let padding = wasapi.audio_client.GetCurrentPadding().unwrap();
            let buffer_size = wasapi.audio_client.GetBufferSize().unwrap();

            let n_frames = buffer_size - 1 - padding;
            assert!(n_frames < buffer_size - padding);

            let size = (n_frames * block_align) as usize;

            if size == 0 {
                continue;
            }

            let b = wasapi.render_client.GetBuffer(n_frames).unwrap();
            let output = std::slice::from_raw_parts_mut(b, size);
            let channels = wasapi.format.Format.nChannels as usize;

            let volume = VOLUME.load(Ordering::Relaxed);
            macro_rules! next {
                () => {
                    if let Some(s) = BUFFER.as_ref().unwrap().get(i) {
                        let s = (s * volume).to_le_bytes();
                        i += 1;
                        s
                    } else {
                        (0.0f32).to_le_bytes()
                    }
                };
            }

            for bytes in output.chunks_mut(std::mem::size_of::<f32>() * channels) {
                bytes[0..4].copy_from_slice(&next!());
                if channels > 1 {
                    bytes[4..8].copy_from_slice(&next!());
                }
            }

            wasapi.render_client.ReleaseBuffer(n_frames, 0).unwrap();

            if WaitForSingleObject(wasapi.event, u32::MAX) != WAIT_OBJECT_0 {
                unreachable!()
            }
        }
    });
}

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    unsafe {
        decoder_thread();
        wasapi_thread();
        // std::thread::sleep_ms(200);

        SEEK = Some(140.0);
        VOLUME.store(0.01, Ordering::Relaxed);

        COMMAND.store(PAUSE, Ordering::Relaxed);

        PATH = Mutex::new(Some(PathBuf::from(
            // r"D:\OneDrive\Music\Steve Reich\Six Pianos Music For Mallet Instruments, Voices And Organ  Variations For Winds, Strings And Keyboards\03 Variations for Winds, Strings and Keyboards.flac",
            r"D:\OneDrive\Music\Various Artists\Death Note - Original Soundtrack\01 Death Note.flac",
        )));
        PATH_CONDVAR.notify_all();

        std::thread::sleep_ms(2000);
        info!("PLAY");
        COMMAND.store(NONE, Ordering::Relaxed);
    }

    thread::park();
}

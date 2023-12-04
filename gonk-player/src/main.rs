use gonk_player::{decoder::Symphonia, *};
use makepad_windows::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
use mini::*;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Condvar, Mutex,
    },
    thread,
    time::Duration,
};

//Two threads should always be running
//The wasapi thread and the decoder thread.
//The wasapi thread reads from a buffer, if the buffer is empty, block until it's not.
//The decoder thread needs a way to request a new file to be read.
//It should read the contents of the audio file into the shared buffer.

use boxcar::Vec;

static mut BUFFER: Option<Vec<f32>> = None;

static mut PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static mut PATH_CONDVAR: Condvar = Condvar::new();
static mut STOP: AtomicBool = AtomicBool::new(false);

static mut ELAPSED: Duration = Duration::from_secs(0);
static mut STATE: State = State::Stopped;

static mut SEEK: Option<f32> = None;
static mut VOLUME: f32 = 0.1;

pub unsafe fn decoder_thread() {
    thread::spawn(|| {
        info!("Spawned decoder thread!");
        let lock = PATH.lock().unwrap();
        let lock = PATH_CONDVAR.wait(lock).unwrap();
        let path = lock.as_ref().unwrap();
        info!("Decoder thread unlocked!");

        let mut sym = Symphonia::new(path).unwrap();
        //TODO: We need to cache every packet length and the timestamp.
        //This way we can calculate the current time but getting the buffer index.
        //Seeking will need to be compeletely redesigned.
        //When seeking with symphonia the mediasourcestream will be updated.
        //This would break my buffer. If the packet is already loaded I want to use it.
        while let Some(packet) = sym.next_packet() {
            BUFFER.as_mut().unwrap().extend(packet.samples().to_vec());
            // HEAD.store(BUFFER.len(), Ordering::Relaxed);
            if let Some(seek) = SEEK {
                //Just clear the whole buffer and rebuild for now.
                // BUFFER.clear();
                sym.seek(seek);
            }

            if STOP.load(Ordering::Relaxed) {
                while !STOP.load(Ordering::Relaxed) {
                    //TODO: Swap to condvar.
                    std::hint::spin_loop();
                }
            }
        }

        //TODO: Add all commands such as new, stop, seek into a command condvar.
        info!("Finished reading file, waiting for a new one.");
        let lock = PATH_CONDVAR.wait(PATH.lock().unwrap());
    });
}

pub unsafe fn wasapi_thread() {
    thread::spawn(|| {
        let default = default_device();
        let wasapi = Wasapi::new(&default, Some(44100)).unwrap();
        let mut i = 0;

        let block_align = wasapi.format.Format.nBlockAlign as u32;

        info!("Starting playback");
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

            macro_rules! next {
                () => {
                    if let Some(s) = BUFFER.as_ref().unwrap().get(i) {
                        let s = (s * VOLUME).to_le_bytes();
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
        BUFFER = Some(Vec::new());
        decoder_thread();
        wasapi_thread();
        std::thread::sleep_ms(200);

        PATH = Mutex::new(Some(PathBuf::from(
            // r"D:\OneDrive\Music\Steve Reich\Six Pianos Music For Mallet Instruments, Voices And Organ  Variations For Winds, Strings And Keyboards\03 Variations for Winds, Strings and Keyboards.flac",
            r"D:\OneDrive\Music\Various Artists\Death Note - Original Soundtrack\01 Death Note.flac",
        )));
        PATH_CONDVAR.notify_all();
    }

    thread::park();
}

#![feature(const_maybe_uninit_zeroed)]
use gonk_player::{decoder::Symphonia, *};
use std::{
    mem::MaybeUninit,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Condvar, Mutex,
    },
    thread,
    time::Duration,
};
use symphonia::core::units::{Time, TimeBase};

//Two threads should always be running

//The wasapi thread and the decoder thread.

//The wasapi thread reads from a buffer, if the buffer is empty, block until it's not.

//The decoder thread needs a way to request a new file to be read.
//It should read the contents of the audio file into the shared buffer.

static mut BUFFER: Vec<f32> = Vec::new();

static mut PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static mut PATH_CONDVAR: Condvar = Condvar::new();
static mut STOP: AtomicBool = AtomicBool::new(false);
static mut ELAPSED: Duration = Duration::from_secs(0);
static mut STATE: State = State::Stopped;
static mut SEEK: Option<f64> = None;

pub unsafe fn decoder_thread() {
    thread::spawn(|| {
        let lock = PATH.lock().unwrap();
        let lock = PATH_CONDVAR.wait(lock).unwrap();
        let path = lock.as_ref().unwrap();
        let mut sym = Symphonia::new(path).unwrap();
        //TODO: We need to cache every packet length and the timestamp.
        //This way we can calculate the current time but getting the buffer index.
        //Seeking will need to be compeletely redesigned.
        //When seeking with symphonia the mediasourcestream will be updated.
        //This would break my buffer. If the packet is already loaded I want to use it.
        while let Some(packet) = sym.next_packet() {
            if let Some(seek) = SEEK {
                // let pos = Duration::from_secs_f64(sym.duration().as_secs_f64() * seek);
                // let params = &sym.track.codec_params;
                // let time = Time::new(pos.as_secs(), pos.subsec_nanos() as f64 / 1_000_000_000.0);

                // if let Some(sample_rate) = params.sample_rate {
                //     let ts = TimeBase::new(1, sample_rate).calc_timestamp(time);
                // } else {
                //     panic!();
                // }
            }

            //
        }

        if STOP.load(Ordering::Relaxed) {}
    });
}

pub fn read<P: AsRef<Path>>(path: P) {}

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    // const N: usize = u16::MAX as usize;
    // unsafe { RB = MaybeUninit::new(Rb::new(N)) }

    // wasapi::set_volume(0.05);

    // wasapi::thread(unsafe { RB.assume_init_mut() });

    // #[rustfmt::skip]
    // let mut sym = Symphonia::new(r"D:\Downloads\Variations for Winds_ Strings and Keyboards - San Francisco Symphony.flac").unwrap();
    // let mut elapsed = Duration::default();
    // let mut state = State::Playing;

    // let rb = unsafe { RB.assume_init_mut() };
    // while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
    //     rb.append(packet.samples());
    //     break;
    // }

    // rb.clear();

    // while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
    //     rb.append(packet.samples());
    // }

    thread::park();
}

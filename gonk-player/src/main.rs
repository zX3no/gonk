use ::log::info;
use gonk_core::*;
use gonk_player::{decoder::Symphonia, static_rb::*, *};
use std::{ptr::addr_of_mut, sync::atomic::Ordering, thread, time::Duration};

//I want to rework the player.

//When a file is requested to be played, a thread is opened and the bytes are read in ASAP.

//The Player keeps track of the index and tries to read the next samples.

//I want a ring buffer that allows me to request somes bytes without re-allocating.

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    // const N: usize = 1024 * 9 * 2;
    // let mut rb = Rb::new(N);

    const N: usize = 1024 * 9;
    let mut rb: StaticRb<N> = StaticRb::<N>::new();
    let ptr = addr_of_mut!(rb) as usize;

    thread::spawn(move || {
        let rb = unsafe { (ptr as *mut StaticRb<N>).as_mut().unwrap() };
        // let rb = unsafe { (ptr as *mut Rb).as_mut().unwrap() };

        // thread::sleep(Duration::from_millis(2000));
        let default = default_device();
        let mut wasapi = Wasapi::new(&default, None).unwrap();
        loop {
            // wasapi.fill_heap(0.1, rb).unwrap();
            wasapi.fill(0.1, rb).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
    });

    #[rustfmt::skip]
    let mut sym = Symphonia::new(r"D:\Downloads\Variations for Winds_ Strings and Keyboards - San Francisco Symphony.flac").unwrap();
    let mut elapsed = Duration::default();
    let mut state = State::Playing;

    while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
        let samples = packet.samples();

        // info!("Adding {} samples.", samples.len());
        rb.append(samples);

        // println!("Pushed samples {}", samples.len());
        // for sample in samples {
        //     rb.push_blocking(*sample);
        // }
    }

    thread::park();
}

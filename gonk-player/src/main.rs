use gonk_core::*;
use gonk_player::{decoder::Symphonia, *};
use std::{ptr::addr_of_mut, thread, time::Duration};

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
    // let mut buffer: Rb<f32, 1024> = Rb::new();
    // let ptr = addr_of_mut!(buffer) as usize;

    // thread::spawn(move || {
    //     let buffer = ptr as *mut Rb<f32, 1024>;
    //     thread::sleep(Duration::from_millis(100));
    //     let _ = unsafe { (*buffer).pop_front() };
    //     // let _ = unsafe { (*buffer).pop_front() };
    // });

    // for i in 0..1024 {
    //     buffer.push_back(i as f32);
    // }
    // buffer.push_back(10.0 as f32);
    // dbg!(buffer.buf.len());
    // buffer.push_back(11.0);
    // buffer.push_back(12.0);
    // buffer.push_back(12.0);
    // buffer.push_back(13.0);

    const N: usize = 4;
    let mut rb: Rb<f32, N> = Rb::new();
    let ptr = addr_of_mut!(rb) as usize;

    thread::spawn(move || {
        let rb = unsafe { (ptr as *mut Rb<f32, N>).as_mut().unwrap() };

        // thread::sleep(Duration::from_millis(2000));
        let default = default_device();
        let mut wasapi = Wasapi::new(&default, None).unwrap();
        wasapi.fill(0.05, rb).unwrap();
    });

    #[rustfmt::skip]
    let mut sym = Symphonia::new(r"D:\Downloads\Variations for Winds_ Strings and Keyboards - San Francisco Symphony.flac").unwrap();
    let mut elapsed = Duration::default();
    let mut state = State::Playing;

    while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
        let samples = packet.samples();

        println!("Pushed samples {}", samples.len());
        for sample in samples {
            rb.push_back(*sample);
        }
    }

    thread::park();
}

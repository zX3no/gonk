use core::slice;
use gonk_core::{profile, profiler};
use gonk_player::decoder::Symphonia;
use gonk_player::*;
use std::{
    collections::VecDeque,
    ptr::{addr_of, addr_of_mut, null_mut},
    sync::Arc,
    thread,
    time::Duration,
};

//I want to rework the player.

//When a file is requested to be played, a thread is opened and the bytes are read in ASAP.

//The Player keeps track of the index and tries to read the next samples.

//I want a ring buffer that allows me to request somes bytes without re-allocating.

fn main() {
    let mut buffer: Rb<f32> = Rb::new(2);
    let ptr = addr_of_mut!(buffer) as usize;

    // thread::spawn(move || {
    //     let buffer = ptr as *mut Rb<f32>;
    //     thread::sleep(Duration::from_millis(100));
    //     let _ = unsafe { (*buffer).pop_front() };
    //     // let _ = unsafe { (*buffer).pop_front() };
    // });

    buffer.push_back(10.0);
    buffer.push_back(11.0);
    // buffer.push_back(12.0);
    // buffer.push_back(13.0);

    profiler::print();

    // thread::park();
    // buffer.push_back(10.0);
    // dbg!(buffer.pop_front());
    // loop {
    //     dbg!(buffer.pop_front());

    //     thread::sleep(Duration::from_millis(16));
    // }

    // let default = default_device();
    // let mut wasapi = Wasapi::new(&default, None).unwrap();

    // let buffer_size = wasapi.buffer_size().unwrap() as usize;

    // let rb: SpscRb<u8> = SpscRb::new(buffer_size * 8);
    // let (prod, cons) = (rb.producer(), rb.consumer());

    // thread::spawn(move || {
    //     #[rustfmt::skip]
    //     let mut sym = Symphonia::new(r"D:\Downloads\Variations for Winds_ Strings and Keyboards - San Francisco Symphony.flac").unwrap();
    //     let mut elapsed = Duration::default();
    //     let mut state = State::Playing;

    //     while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
    //         let bytes = unsafe { packet.samples().align_to::<u8>().1 };
    //         prod.write_blocking(bytes).unwrap();
    //     }
    // });

    // let mut buf = vec![0; buffer_size];
    // while let Some(_) = cons.read_blocking(&mut buf) {
    //     wasapi.fill(0.05, &buf).unwrap();
    // }

    // thread::park();
}

use core::ptr::addr_of_mut;
use std::thread;

use gonk_player::{decoder::*, default_device, init, Wasapi};
use ringbuf::HeapRb;

fn main() {
    let capacity = ((20 * 44100) / 1000) * 2;
    let rb = HeapRb::<f32>::new(capacity);
    let (mut prod, mut cons) = rb.split();

    let mut decoder = Decoder::new(r"D:\OneDrive\Music\Sam Gellaitry\Viewfinder Vol. 1 PHOSPHENE\04. Sam Gellaitry - Neptune.flac").unwrap();

    let cast: usize = addr_of_mut!(decoder.symphonia) as usize;
    thread::spawn(move || {
        let sym = cast as *mut Symphonia;
        loop {
            unsafe {
                // if BUFFER.len() < ((20 * 44100) / 1000) * 2 {
                if prod.len() < capacity {
                    if let Some(packet) = (*sym).next_packet() {
                        prod.push_slice(packet.samples());
                    }
                }
                // }
            }
        }
    });

    init();
    let default = default_device().unwrap();
    let mut wasapi = unsafe { Wasapi::new(default, Some(44100)) };
    loop {
        unsafe {
            wasapi.fill_buffer(&mut decoder, 0.8, &mut cons);
        }
    }
}

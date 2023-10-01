#![feature(const_maybe_uninit_zeroed)]
use gonk_player::{decoder::Symphonia, *};
use std::{mem::MaybeUninit, thread, time::Duration};

static mut RB: MaybeUninit<Rb> = MaybeUninit::zeroed();

static mut BUFFER: Vec<f32> = Vec::new();

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    const N: usize = u16::MAX as usize;
    unsafe { RB = MaybeUninit::new(Rb::new(N)) }

    wasapi::set_volume(0.05);

    wasapi::create(unsafe { RB.assume_init_mut() });

    #[rustfmt::skip]
    let mut sym = Symphonia::new(r"D:\Downloads\Variations for Winds_ Strings and Keyboards - San Francisco Symphony.flac").unwrap();
    let mut elapsed = Duration::default();
    let mut state = State::Playing;

    let rb = unsafe { RB.assume_init_mut() };
    while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
        rb.append(packet.samples());
        break;
    }

    rb.clear();

    while let Some(packet) = sym.next_packet(&mut elapsed, &mut state) {
        rb.append(packet.samples());
    }

    thread::park();
}

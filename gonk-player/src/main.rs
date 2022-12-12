use gonk_player::{
    decoder::{self},
    default_device, init, Wasapi,
};

fn main() {
    let path = r"D:\OneDrive\Music\Sam Gellaitry\Viewfinder Vol. 1 PHOSPHENE\04. Sam Gellaitry - Neptune.flac";
    let mut decoder = decoder::new_leak(path).unwrap();

    init();
    let default = default_device().unwrap();
    let mut wasapi = unsafe { Wasapi::new(default, Some(44100)) };
    loop {
        unsafe {
            // let now = Instant::now();
            wasapi.fill_buffer(&mut decoder, 1.0);
            // dbg!(now.elapsed());
        }
    }
}

use gonk_player::{decoder::Decoder, default_device, init, Wasapi};

fn main() {
    init();

    let path = r"D:\OneDrive\Music\Sam Gellaitry\Viewfinder Vol. 1 PHOSPHENE\04. Sam Gellaitry - Neptune.flac";
    // let mut decoder = decoder::new(path).unwrap();
    let mut decoder = Decoder::new(path).unwrap();

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

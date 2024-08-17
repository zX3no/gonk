pub use gonk_core::*;
use gonk_player::*;

fn main() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let device = default_device();
    spawn_audio_threads(device);
    set_volume(5);
    play_path(r"D:\Downloads\test.flac");

    std::thread::park();
}

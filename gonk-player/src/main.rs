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

    volume_down();
    volume_down();

    // let path = r"D:\OneDrive\Music\Steve Reich\Six Pianos Music For Mallet Instruments, Voices And Organ  Variations For Winds, Strings And Keyboards\03 Variations for Winds, Strings and Keyboards.flac";

    let path1 =
        r"D:\OneDrive\Music\Various Artists\Death Note - Original Soundtrack\01 Death Note.flac";
    let path2 =
        r"\\?\D:\OneDrive\Music\Iglooghost\░░░ Fracture Vault ☼⑇\02 Bruise Swamp『YOLK TWEAK』.mp3";
    play(path1);
    // std::thread::sleep_ms(200);
    // play(path2);

    std::thread::park();
}

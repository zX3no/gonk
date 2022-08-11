use gonk_player::*;
use std::thread;

fn main() {
    let player = Player::new(15);

    // let path = r"D:\OneDrive\Music\Foxtails\fawn\09. life is a death scene, princess.flac";
    let path = r"D:\OneDrive\Music\Foxtails\fawn\06. gallons of spiders went flying thru the stratosphere.flac";
    player.play(path);
    player.seek_to(172.0);

    thread::park();
}

use rodio::*;
use std::io::BufReader;

fn main() {
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();

    let file = std::fs::File::open("examples/music.ogg").unwrap();
    sink.append(Decoder::new_decoder(BufReader::new(file)).unwrap());
    //Seeks don't work?
    // sink.seek(time::Duration::from_millis(10));
    sink.sleep_until_end();
}

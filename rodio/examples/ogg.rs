use rodio::*;
use std::{fs::File, io::BufReader, path::Path};

fn main() {
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();

    let path = Path::new("examples/music.ogg");
    let file = File::open(path).unwrap();
    sink.append(Decoder::new_decoder(BufReader::new(file), path.extension()).unwrap());
    //Seeks don't work?
    // sink.seek(time::Duration::from_millis(10));
    sink.sleep_until_end();
}

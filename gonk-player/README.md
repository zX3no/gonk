## New Design

Input -> (Play song at path ...)

API -> Decode Audio -> Send audio to operating system

```rust
static mut WASAPI: Wasapi = Lazy::new(|| Wasapi::new());


struct Player {
    songs: Index<Song>,
    decoder: Decoder,
}

impl Player {
    fn push(&self, songs: &[Song]) {
        //add songs to queue
    }
    fn next(&self) {
        self.songs.next();
        self.decoder.load(self.songs.playing())
    }
    fn play() {
        self.decoder.load("D:/Test.flac");
    }
    fn seek() {
        self.decoder.seek("+30 seconds");
    }
}

struct Decoder {
    symphonia: Arc<RwLock<Symphonia>>,
    //A new ring buffer will need to be constructed every song.
    //Since the sample rates vary, the size required to buffer 20ms will change.
    ringbuf: Ringbuf,
}

impl Decoder {
    fn new() {
        thread::spawn(|| {
            //Push samples into the buffer.
            ringbuf.push(syphonia.decode());
        });
    }
    fn seek(&self) {
        self.symphonia.seek();
        self.ringbuf.clear();
    }
}

WASAPI.set_output_device("Output 3");
WASAPI.set_sample_rate(44100);
```


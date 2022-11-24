## New Design

Input -> (Play song at path ...)

API -> Decode Audio -> Send audio to operating system

```rust
static mut DECODER: Decoder = Lazy::new(|| Decoder::new());

static mut WASAPI: Wasapi = Lazy::new(|| Wasapi::new());

struct Player {
    songs: Index<Song>,
}

impl Player {
    fn push(&self, songs: &[Song]) {
        //add songs to queue
    }
    fn next(&self) {
        self.songs.next();
        DECODER.load(self.songs.playing())
    }
    fn play() {
        DECODER.load("D:/Test.flac");
    }
    fn seek() {
        DECODER.seek("+30 seconds");
    }
}

let packets = DECODER.next();

WASAPI.set_output_device("Output 3");
WASAPI.set_sample_rate(44100);
```


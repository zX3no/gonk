use gonk_database::*;

fn main() {
    init();

    // let song = RawSong::new(
    //     "joe",
    //     "joe's album",
    //     "joe's title",
    //     "D:\\OneDrive\\Joe\\joe's song.flac",
    //     2,
    //     1,
    // );

    let songs = songs_from_album("Arca", "Xen");
    dbg!(songs);

    bench_slow(|| {
        let _songs = songs_from_album("Arca", "Xen");
    });
}

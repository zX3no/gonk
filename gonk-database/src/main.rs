#![allow(unused)]
use gonk_database::*;

fn main() {
    init();

    let _song = RawSong::new(
        "joe",
        "joe's album",
        "joe's title",
        "D:\\OneDrive\\Joe\\joe's song.flac",
        20,
        1,
        0.01,
    );
    let songs = songs_from_album(
        "Various Artists",
        "The Legend of Zelda: Breath of the Wild Original Soundtrack",
    );
    let ids: Vec<usize> = songs.iter().map(|song| song.id).collect();

    bench_slow(|| {
        let _songs = albums_by_artist("Death Grips");
    });

    bench_slow(|| {
        let _songs = albums_by_artist("Death Grips");
    });
}

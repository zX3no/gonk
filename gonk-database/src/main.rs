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

    let songs = songs_from_album("Arca", "Xen");
    dbg!(songs);
}

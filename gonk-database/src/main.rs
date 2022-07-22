use gonk_database::*;

fn main() {
    init();

    let song = RawSong::new(
        "joe's song",
        "joe's album",
        "joe",
        "D:\\OneDrive\\Joe\\joe's song.flac",
        2,
        1,
    );
    let result = scan_text(&song.text);
    dbg!(result);

    bench(|| {
        let result = scan_text(&song.text);
        // let name = name(&song.text);
        // let album = album(&song.text);
        // let artist = artist(&song.text);
        // let path = path(&song.text);
        // assert!(!name.is_empty())
    });
}

use gonk_core::*;

fn main() {
    let song =
        read_metadata(r"D:\OneDrive\Music\Joji\Chloe Burbank Vol. 1\01 Medicine (Remix).flac")
            .unwrap();
    dbg!(song);
}

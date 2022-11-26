#![allow(dead_code)]

// #[repr(packed)]
#[derive(Default, Debug)]
pub struct Song {
    pub text: Text,
    pub number: u8,
    pub disc: u8,
    pub gain: f32,
    pub padding: Vec<u8>,
}

// #[repr(packed)]
#[derive(Default, Debug)]
pub struct Text {
    pub artist_len: u16,
    pub album_len: u16,
    pub title_len: u16,
    pub path_len: u16,
    pub artist: &'static str,
    pub album: &'static str,
    pub title: &'static str,
    pub path: &'static str,
    pub padding: Vec<u8>,
}

#[derive(Default, Debug)]
pub struct S {
    pub text: T,
    pub number: u8,
    pub disc: u8,
    pub gain: f32,
}

impl S {
    pub fn as_bytes(&self) -> Vec<u8> {
        [
            self.text.as_bytes().as_slice(),
            &[self.number, self.disc],
            self.gain.to_le_bytes().as_slice(),
        ]
        .concat()
    }
}

#[derive(Default, Debug)]
pub struct T {
    pub artist: &'static str,
    pub album: &'static str,
    pub title: &'static str,
    pub path: &'static str,
}

impl T {
    pub fn as_bytes(&self) -> Vec<u8> {
        [
            (self.artist.len() as u16).to_le_bytes().as_slice(),
            (self.album.len() as u16).to_le_bytes().as_slice(),
            (self.title.len() as u16).to_le_bytes().as_slice(),
            (self.path.len() as u16).to_le_bytes().as_slice(),
            self.artist.as_bytes(),
            self.album.as_bytes(),
            self.title.as_bytes(),
            self.path.as_bytes(),
        ]
        .concat()
    }
}

pub const unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

fn main() {
    let song = Song {
        text: Text {
            artist_len: "artist".len() as u16,
            album_len: "album".len() as u16,
            title_len: "title".len() as u16,
            path_len: "path".len() as u16,
            artist: "artist",
            album: "album",
            title: "title",
            path: "path",
            padding: Vec::new(),
        },
        number: 1,
        disc: 1,
        gain: 1.0,
        padding: Vec::new(),
    };

    let bytes: &[u8] = unsafe { any_as_u8_slice(&song) };
    let pointer: *const [u8; std::mem::size_of::<Song>()] =
        bytes as *const _ as *const [u8; std::mem::size_of::<Song>()];
    let song: Song = unsafe { std::mem::transmute(*pointer) };

    dbg!(song);
}

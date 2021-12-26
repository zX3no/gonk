use std::{
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::Result;
fn read(stream: &mut TcpStream) -> Result<()> {
    let mut rx_bytes = [0u8; 9999];
    stream.read(&mut rx_bytes)?;
    let received = std::str::from_utf8(&rx_bytes).expect("valid utf8");
    eprintln!("{}", received);
    Ok(())
}

fn main() -> Result<()> {
    if let Ok(mut stream) = TcpStream::connect("192.168.0.100:6600") {
        read(&mut stream)?;
        // get_artists(&mut stream);
        get_albums_by_artist(&mut stream);
    }

    Ok(())
}

fn get_artists(stream: &mut TcpStream) {
    stream.write_all(b"list albumartist\n").unwrap();
    read(stream).unwrap();
}

fn get_albums_by_artist(stream: &mut TcpStream) {
    stream
        .write_all(b"list album (albumartist=='Arca')\n")
        .unwrap();
    read(stream).unwrap();
}

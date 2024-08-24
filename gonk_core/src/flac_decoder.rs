use crate::{db::UNKNOWN_ARTIST, Song};
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    str::from_utf8_unchecked,
};

#[inline]
pub fn u24_be(reader: &mut BufReader<File>) -> u32 {
    let mut triple = [0; 4];
    reader.read_exact(&mut triple[0..3]).unwrap();
    u32::from_be_bytes(triple) >> 8
}

#[inline]
pub fn u32_le(reader: &mut BufReader<File>) -> u32 {
    let mut buffer = [0; 4];
    reader.read_exact(&mut buffer).unwrap();
    u32::from_le_bytes(buffer)
}

pub fn read_metadata_old<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut flac = [0; 4];
    reader.read_exact(&mut flac)?;

    if unsafe { from_utf8_unchecked(&flac) } != "fLaC" {
        Err("File is not FLAC.")?;
    }

    let mut tags = HashMap::new();

    loop {
        let mut flag = [0; 1];
        reader.read_exact(&mut flag)?;

        // First bit of the header indicates if this is the last metadata block.
        let is_last = (flag[0] & 0x80) == 0x80;

        // The next 7 bits of the header indicates the block type.
        let block_type = flag[0] & 0x7f;
        let block_len = u24_be(&mut reader);

        //VorbisComment https://www.xiph.org/vorbis/doc/v-comment.html
        if block_type == 4 {
            let vendor_length = u32_le(&mut reader);
            reader.seek_relative(vendor_length as i64)?;

            let comment_list_length = u32_le(&mut reader);
            for _ in 0..comment_list_length {
                let length = u32_le(&mut reader) as usize;
                let mut buffer = vec![0; length as usize];
                reader.read_exact(&mut buffer)?;

                let tag = core::str::from_utf8(&buffer).unwrap();
                let (k, v) = match tag.split_once('=') {
                    Some((left, right)) => (left, right),
                    None => (tag, ""),
                };

                tags.insert(k.to_ascii_uppercase(), v.to_string());
            }

            return Ok(tags);
        }

        reader.seek_relative(block_len as i64)?;

        // Exit when the last header is read.
        if is_last {
            break;
        }
    }

    Err("Could not parse metadata.")?
}

pub fn read_metadata<P: AsRef<Path>>(path: P) -> Result<Song, Box<dyn Error>> {
    let file = File::open(&path)?;
    let mut reader = BufReader::new(file);

    let mut flac = [0; 4];
    reader.read_exact(&mut flac)?;

    if unsafe { from_utf8_unchecked(&flac) } != "fLaC" {
        Err("File is not FLAC.")?;
    }

    let mut song: Song = Song::default();
    song.path = path.as_ref().to_string_lossy().to_string();

    let mut flag = [0; 1];

    loop {
        reader.read_exact(&mut flag)?;

        // First bit of the header indicates if this is the last metadata block.
        let is_last = (flag[0] & 0x80) == 0x80;

        // The next 7 bits of the header indicates the block type.
        let block_type = flag[0] & 0x7f;
        let block_len = u24_be(&mut reader);

        //VorbisComment https://www.xiph.org/vorbis/doc/v-comment.html
        if block_type == 4 {
            let vendor_length = u32_le(&mut reader);
            reader.seek_relative(vendor_length as i64)?;

            let comment_list_length = u32_le(&mut reader);
            for _ in 0..comment_list_length {
                let length = u32_le(&mut reader) as usize;
                let mut buffer = vec![0; length as usize];
                reader.read_exact(&mut buffer)?;

                let tag = core::str::from_utf8(&buffer).unwrap();
                let (k, v) = match tag.split_once('=') {
                    Some((left, right)) => (left, right),
                    None => (tag, ""),
                };

                match k.to_ascii_lowercase().as_str() {
                    "albumartist" => song.artist = v.to_string(),
                    "artist" if song.artist == UNKNOWN_ARTIST => song.artist = v.to_string(),
                    "title" => song.title = v.to_string(),
                    "album" => song.album = v.to_string(),
                    "tracknumber" => song.track_number = v.parse().unwrap_or(1),
                    "discnumber" => song.disc_number = v.parse().unwrap_or(1),
                    "replaygain_track_gain" => {
                        //Remove the trailing " dB" from "-5.39 dB".
                        if let Some(slice) = v.get(..v.len() - 3) {
                            if let Ok(db) = slice.parse::<f32>() {
                                song.gain = 10.0f32.powf(db / 20.0);
                            }
                        }
                    }
                    _ => {}
                }
            }

            return Ok(song);
        }

        reader.seek_relative(block_len as i64)?;

        // Exit when the last header is read.
        if is_last {
            break;
        }
    }

    Err("Could not parse metadata.")?
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test() {
        const PATH: &str = "D:\\OneDrive\\Music";

        let paths: Vec<winwalk::DirEntry> = winwalk::walkdir(PATH, 0)
            .into_iter()
            .flatten()
            .filter(|entry| match entry.extension() {
                Some(ex) => {
                    matches!(ex.to_str(), Some("flac"))
                }
                None => false,
            })
            .collect();

        let songs: Vec<Result<Song, String>> = paths
            .iter()
            .map(|file| {
                read_metadata(&file.path)
                    .map_err(|err| format!("Error: ({err}) @ {}", file.path.to_string()))
            })
            .collect();

        dbg!(&songs[0].as_ref().unwrap());
    }
}

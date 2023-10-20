//! FLAC decoder
//!
//! Currently only supports reading metadata.
//!
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    str::from_utf8_unchecked,
};

pub fn u24_be(reader: &mut BufReader<File>) -> u32 {
    let mut triple = [0; 4];
    reader.read_exact(&mut triple[0..3]).unwrap();
    u32::from_be_bytes(triple) >> 8
}

pub fn u32_le(reader: &mut BufReader<File>) -> u32 {
    let mut buffer = [0; 4];
    reader.read_exact(&mut buffer).unwrap();
    u32::from_le_bytes(buffer)
}

pub fn read_metadata<P: AsRef<Path>>(path: P) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut flac = [0; 4];
    reader.read_exact(&mut flac)?;
    let flac = unsafe { from_utf8_unchecked(&flac) };
    if flac != "fLaC" {
        return Err("File is not FLAC.")?;
    }

    let mut tags = HashMap::new();

    loop {
        let mut flag = [0; 1];
        reader.read_exact(&mut flag)?;
        let flag = flag[0];

        // First bit of the header indicates if this is the last metadata block.
        let is_last = (flag & 0x80) == 0x80;

        // The next 7 bits of the header indicates the block type.
        let block_type = flag & 0x7f;

        let block_len = u24_be(&mut reader);

        match block_type {
            //StreamInfo
            0 => reader.seek_relative(block_len as i64)?,
            //Padding
            1 => reader.seek_relative(block_len as i64)?,
            //Application
            2 => reader.seek_relative(block_len as i64)?,
            //SeekTable
            3 => reader.seek_relative(block_len as i64)?,
            //VorbisComment https://www.xiph.org/vorbis/doc/v-comment.html
            4 => {
                /*
                  1) [vendor_length] = read an unsigned integer of 32 bits
                  2) [vendor_string] = read a UTF-8 vector as [vendor_length] octets
                  3) [user_comment_list_length] = read an unsigned integer of 32 bits
                  4) iterate [user_comment_list_length] times {
                       5) [length] = read an unsigned integer of 32 bits
                       6) this iteration's user comment = read a UTF-8 vector as [length] octets
                     }
                  7) [framing_bit] = read a single bit as boolean
                  8) if ( [framing_bit] unset or end of packet ) then ERROR
                  9) done.
                */
                let vendor_length = u32_le(&mut reader);
                reader.seek_relative(vendor_length as i64)?;
                let comment_list_length = u32_le(&mut reader);

                for _ in 0..comment_list_length {
                    let length = u32_le(&mut reader);
                    let mut buffer = vec![0; length as usize];
                    reader.read_exact(&mut buffer)?;

                    let tag = String::from_utf8_lossy(&buffer);

                    let t: Vec<&str> = tag.splitn(2, '=').collect();
                    let (k, v) = if t.len() != 2 {
                        (t[0], "")
                    } else {
                        (t[0], t[1])
                    };

                    tags.insert(k.to_ascii_uppercase(), v.to_string());
                }
                return Ok(tags);
            }
            //Cuesheet
            5 => reader.seek_relative(block_len as i64)?,
            //Picture - Ignore for now.
            6 => reader.seek_relative(block_len as i64)?,
            //Unknown
            _ => reader.seek_relative(block_len as i64)?,
        };

        // Exit when the last header is read.
        if is_last {
            break;
        }
    }

    Err("Could not parse metadata.")?
}

use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufReader, Read},
    mem,
    path::Path,
    str::from_utf8_unchecked,
};

pub fn u16_be(reader: &mut BufReader<File>) -> u16 {
    let mut double_bytes = [0; 2];
    reader.read_exact(&mut double_bytes).unwrap();
    u16::from_be_bytes(double_bytes)
}

pub fn u24_be(reader: &mut BufReader<File>) -> u32 {
    let mut triple = [0; 3];
    reader.read_exact(&mut triple).unwrap();

    let mut buffer = [0u8; mem::size_of::<u32>()];
    buffer[0..3].clone_from_slice(&triple);
    u32::from_be_bytes(buffer) >> 8
}

pub fn u32_le(reader: &mut BufReader<File>) -> u32 {
    let mut buffer = [0; 4];
    reader.read_exact(&mut buffer).unwrap();
    u32::from_le_bytes(buffer)
}

pub fn u64_be(reader: &mut BufReader<File>) -> u64 {
    let mut buffer = [0; 8];
    reader.read_exact(&mut buffer).unwrap();
    u64::from_be_bytes(buffer)
}

//a * (2^(length of b in bits)) + b = a concat b
pub const fn concat_big(a: u64, b: u64, b_len: u8) -> u64 {
    (a * (2u64.pow(b_len as u32))) + b
}

pub const fn concat(a: u16, b: u16, b_len: u32) -> u32 {
    a as u32 * (2u32.pow(b_len)) + b as u32
}

#[derive(Debug, Default)]
pub struct StreamInfo {
    /// The minimum and maximum number of decoded samples per block of audio.
    pub block_len_min: u16,
    pub block_len_max: u16,
    /// The minimum and maximum byte length of an encoded block (frame) of audio. Either value may
    /// be 0 if unknown.
    pub frame_byte_len_min: u32,
    pub frame_byte_len_max: u32,
    /// The sample rate in Hz.
    pub sample_rate: u32,
    /// The channel mask.
    pub channels: u8,
    /// The number of bits per sample of the stream.
    pub bits_per_sample: u8,
    /// The total number of samples in the stream, if available.
    pub n_samples: Option<u64>,
    /// The MD5 hash value of the decoded audio.
    pub md5: [u8; 16],
}

#[derive(Debug)]
pub struct SeekPoint {
    /// The frame or sample timestamp of the `SeekPoint`.
    pub sample_ts: u64,
    /// The byte offset of the `SeekPoint`s timestamp relative to a format-specific location.
    pub offset: u64,
    /// The number of frames the `SeekPoint` covers.
    pub n_samples: u16,
}

pub fn read_metadata(path: impl AsRef<Path>) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut flac = [0; 4];
    reader.read_exact(&mut flac)?;
    let flac = unsafe { from_utf8_unchecked(&flac) };
    if flac != "fLaC" {
        return Err("File is not FLAC.")?;
    }

    //---Metadata https://xiph.org/flac/format.html#metadata_block
    let mut seek_table = Vec::new();
    let mut stream_info = StreamInfo::default();
    let mut tags = HashMap::new();

    loop {
        //---Metadata Header
        let mut flag = [0; 1];
        reader.read_exact(&mut flag)?;
        let flag = flag[0];

        // First bit of the header indicates if this is the last metadata block.
        let is_last = (flag & 0x80) == 0x80;

        // The next 7 bits of the header indicates the block type.
        let block_type = flag & 0x7f;

        let block_len = u24_be(&mut reader);
        //---End Metadata Header

        match block_type {
            //StreamInfo
            0 => {
                assert_eq!(block_len, 34);

                stream_info.block_len_min = u16_be(&mut reader);
                stream_info.block_len_max = u16_be(&mut reader);

                if stream_info.block_len_min < 16 || stream_info.block_len_max < 16 {
                    return Err("flac: minimum block length is 16 samples")?;
                }

                if stream_info.block_len_max < stream_info.block_len_min {
                    return Err(
                        "flac: maximum block length is less than the minimum block length",
                    )?;
                }

                stream_info.frame_byte_len_min = u24_be(&mut reader);
                stream_info.frame_byte_len_max = u24_be(&mut reader);

                // unknown. Valid values are in the range [0, (2^24) - 1] bytes.
                if stream_info.frame_byte_len_min > 0
                    && stream_info.frame_byte_len_max > 0
                    && stream_info.frame_byte_len_max < stream_info.frame_byte_len_min
                {
                    return Err(
                        "flac: maximum frame length is less than the minimum frame length",
                    )?;
                }

                let mut bits64 = [0; 8];
                reader.read_exact(&mut bits64)?;

                //Sample rate 20 bits index [0, 1] 16 and some of [2] 4
                let mut buf = [0; mem::size_of::<u32>()];
                buf[0..3].clone_from_slice(&bits64[0..3]);
                stream_info.sample_rate = u32::from_be_bytes(buf) >> 12;

                //Number of channels 3 bits
                //Index is the the last 3 bits of [2]
                stream_info.channels = (bits64[2] & 0b0001111 >> 1) + 1;

                if stream_info.channels < 1 || stream_info.channels > 8 {
                    return Err("flac: stream channels are out of bounds")?;
                }

                //single bit
                let last_bit = bits64[2] >> 7;
                //first 4 bits
                let four_bits = bits64[3] >> 4;
                stream_info.bits_per_sample =
                    concat(last_bit as u16, four_bits as u16, 4) as u8 + 1;

                if stream_info.bits_per_sample < 4 || stream_info.bits_per_sample > 32 {
                    return Err("flac: stream bits per sample are out of bounds")?;
                }

                //Total samples in stream 36 bits
                //8 - 4         = 4
                //8 + 8 + 8 + 8 = 32
                let last_4 = bits64[3] & 0b0001111;
                let add = concat_big(last_4 as u64, bits64[4] as u64, 8);
                let add = concat_big(add, bits64[5] as u64, 8);
                let add = concat_big(add, bits64[6] as u64, 8);
                let add = concat_big(add, bits64[7] as u64, 8);

                if add == 0 {
                    stream_info.n_samples = None;
                } else {
                    stream_info.n_samples = Some(add);
                }

                //128 md5 signature
                //TODO: is seeking faster than just reading?
                let mut md5 = [0; 16];
                reader.read_exact(&mut md5)?;
                stream_info.md5 = md5;
            }
            //Padding
            1 => reader.seek_relative(block_len as i64)?,
            //Application
            2 => reader.seek_relative(block_len as i64)?,
            //SeekTable
            3 => {
                let seek_points = block_len / 18;
                for _ in 0..seek_points {
                    let sample_ts = u64_be(&mut reader);
                    if sample_ts != 0xFFFFFFFFFFFFFFFF {
                        let offset = u64_be(&mut reader);
                        let n_samples = u16_be(&mut reader);
                        seek_table.push(SeekPoint {
                            sample_ts,
                            offset,
                            n_samples,
                        })
                    } else {
                        reader.seek_relative(8 + 2)?;
                    }
                }
            }
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
    //---End Metadata

    Ok(tags)
}

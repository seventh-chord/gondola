
//! Loading .wav files

use std::fs::File;
use std::path::Path;
use std::io::{self, Read};
use std::error;
use std::fmt;

const HEADER_SIZE: usize = 44;

pub fn load<P: AsRef<Path>>(path: P) -> Result<(), WavError> {
    let path = path.as_ref();
    let mut file = File::open(path)?;

    let mut header = [0u8; HEADER_SIZE];
    match file.read_exact(&mut header) {
        Ok(()) => {},
        Err(err) => {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                return Err(WavError::InvalidHeader);
            } else {
                return Err(WavError::Io(err));
            }
        },
    }

    // There are some magic numbers in the header, check for those
    let mut bad = false;
    bad |= &header[0..4]   != b"RIFF"; 
    bad |= &header[8..12]  != b"WAVE"; 
    bad |= &header[12..15] != b"fmt"; 
    bad |= &header[36..40] != b"data";
    if bad {
        return Err(WavError::InvalidHeader);
    }

    #[inline(always)]
    fn get_u32(slice: &[u8]) -> u32 {
        ((slice[0] as u32) << 0x00) |
        ((slice[1] as u32) << 0x08) |
        ((slice[2] as u32) << 0x10) |
        ((slice[3] as u32) << 0x18)
    }

    #[inline(always)]
    fn get_u16(slice: &[u8]) -> u16 {
        ((slice[0] as u16) << 0x00) |
        ((slice[1] as u16) << 0x08)
    }

    let file_size        = get_u32(&header[4..]);
    let format_length    = get_u32(&header[16..]);
    let type_format      = get_u16(&header[20..]) as u32;
    let channels         = get_u16(&header[22..]) as u32;
    let sample_rate      = get_u32(&header[24..]);
    let bytes_per_second = get_u32(&header[28..]);
    let bytes_per_frame  = get_u16(&header[32..]) as u32;
    let bits_per_sample  = get_u16(&header[34..]) as u32;
    let data_size        = get_u32(&header[40..]);

    // Check if the values in the header are coherent
    let mut bad = false;
    bad |= bits_per_sample%8 != 0;
    bad |= bytes_per_second != bytes_per_frame*sample_rate;
    bad |= (bits_per_sample/8)*channels != bytes_per_frame;
    if bad {
        return Err(WavError::InvalidHeader);
    }

    println!("format_length = {}", format_length);
    println!("type_format = {}", type_format);
    println!("file_size = {}", file_size);
    println!("data_size = {}", data_size);

    return Ok(());
}

#[derive(Debug)]
pub enum WavError {
    Io(io::Error),
    InvalidHeader,
}

impl error::Error for WavError {
    fn description(&self) -> &str {
        match *self {
            WavError::Io(ref inner) => inner.description(),
            WavError::InvalidHeader => "Invalid WAV header",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            WavError::Io(ref inner) => inner.cause(),
            _ => None,
        }
    }
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WavError::Io(ref inner) => write!(f, "IO error while loading wav file: {}", inner),
            WavError::InvalidHeader => write!(f, "Invalid header"),
        }
    }
}

impl From<io::Error> for WavError {
    fn from(error: io::Error) -> WavError {
        WavError::Io(error)
    }
}

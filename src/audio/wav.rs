
//! Loading .wav files

use std::fs::File;
use std::path::Path;
use std::io::{self, Read};
use std::error;
use std::fmt;
use std::mem;
use std::slice;

use super::*;

const HEADER_SIZE: usize = 44;

pub fn load<P: AsRef<Path>>(path: P) -> Result<AudioBuffer, WavError> {
    let path = path.as_ref();
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;

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

    let channels         = get_u16(&header[22..]) as usize;
    let sample_rate      = get_u32(&header[24..]) as usize;
    let bytes_per_second = get_u32(&header[28..]) as usize;
    let bytes_per_frame  = get_u16(&header[32..]) as usize;
    let bits_per_sample  = get_u16(&header[34..]) as usize;
    let bytes_per_sample = (bits_per_sample / 8) as usize;
    let file_size        = get_u32(&header[4..]) as usize + 8;
    let data_bytes       = get_u32(&header[40..]) as usize;

    // Check if the values in the header are coherent
    let mut bad = false;
    bad |= bits_per_sample%8 != 0; // Ensure each sample is a whole number of bytes
    bad |= !(bytes_per_sample == mem::size_of::<u8>() || bytes_per_sample == mem::size_of::<i16>());
    bad |= bytes_per_second != bytes_per_frame*sample_rate;
    bad |= bytes_per_sample*channels != bytes_per_frame;
    bad |= file_size != data_bytes+HEADER_SIZE;
    bad |= metadata.len() as usize != file_size;
    bad |= data_bytes % (bytes_per_frame as usize) != 0;
    bad |= get_u32(&header[16..]) != 16;
    bad |= get_u16(&header[20..]) != 1; // PCM data
    if bad {
        return Err(WavError::InvalidHeader);
    }

    // Read the data from the file
    let sample_count = data_bytes / bytes_per_sample;
    
    let data = match bytes_per_sample {
        // i16
        2 => {
            let mut samples = Vec::<i16>::with_capacity(sample_count);
            unsafe { samples.set_len(sample_count) };

            {
                let slice = &mut samples[..];
                let ptr = slice.as_mut_ptr() as *mut u8;
                let len = slice.len() / mem::size_of::<i16>();
                let byte_slice = unsafe { slice::from_raw_parts_mut(ptr, len) };

                file.read_exact(byte_slice)?;
            }

            if cfg!(target_endian = "big") {
                // This is slow, but never really happens because x86 chips are little endian
                for sample in samples.iter_mut() {
                    *sample = sample.swap_bytes();
                }
            }

            samples
        },

        // u8
        1 => {
            let mut u8_samples = Vec::<u8>::with_capacity(sample_count);
            unsafe { u8_samples.set_len(sample_count) };
            file.read_exact(&mut u8_samples[..])?;

            // Convert to i16 samples
            let min  = i16::min_value();
            let step = 0x0101;

            let mut i16_samples = Vec::<i16>::with_capacity(sample_count);
            for &sample in u8_samples.iter() {
                let converted = min + (sample as i16)*step;
                i16_samples.push(converted);
            }

            i16_samples
        },

        _ => unreachable!()
    };

    drop(file); // Closes the file

    return Ok(AudioBuffer {
        channels: channels as u8,
        sample_rate: sample_rate as u32,
        data,
    });
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


extern crate winapi;
extern crate kernel32;

use std::mem;
use std::slice;
use std::ptr;

use super::*;

// We access all ffi stuff through `ffi::whatever` instead of through each apis specific
// bindings. This allows us to easily add custom stuff that is missing in bindings.
mod ffi {
    #![allow(non_camel_case_types)]

    pub(super) use super::winapi::*;
    pub(super) use super::kernel32::*;

    // Direct-sound functions
    pub(super) type DirectSoundCreate = extern "system" fn(LPGUID, *mut LPDIRECTSOUND, LPUNKNOWN) -> HRESULT;
}

pub(super) struct AudioBackend {
    buffer_size: usize, // Total size of secondary buffer, in bytes
    playing: bool,
    secondary_buffer: &'static mut ffi::IDirectSoundBuffer,
}

impl AudioBackend {
    pub fn initialize(
        window: &Window,
        sample_rate: u32,
        duration_in_frames: u32,
    ) -> Option<AudioBackend> 
    {
        // Load library
        let lib_name = encode_wide("dsound.dll");
        let dsound_lib = unsafe { ffi::LoadLibraryW(lib_name.as_ptr()) };

        if dsound_lib.is_null() {
            println!("Could not load \"dsound.dll\"");
            // Don't panic, just run without sound
            return None;
        }

        // Create DirectSound object
        let direct_sound_create = {
            let name = b"DirectSoundCreate\0";
            let address = unsafe { ffi::GetProcAddress(dsound_lib, name.as_ptr() as *const _) };

            if address.is_null() {
                println!("Could not load DirectSoundCreate from dsound.dll");
                return None;
            } else {
                unsafe { mem::transmute::<_, ffi::DirectSoundCreate>(address) }
            }
        };

        let mut dsound: ffi::LPDIRECTSOUND = ptr::null_mut();
        let result = direct_sound_create(ptr::null_mut(), &mut dsound, ptr::null_mut());
        if result != ffi::DS_OK {
            println!("Failed to create a direct sound object: {}", result);
            return None;
        }
        assert!(!dsound.is_null());
        let dsound = unsafe { &mut *dsound };

        let result = unsafe { dsound.SetCooperativeLevel(window.window_handle(), ffi::DSSCL_PRIORITY) };
        if result != ffi::DS_OK {
            println!("Failed to call DirectSound->SetCooperativeLevel. Error code: {}", result);
            return None;
        }

        // Create primary buffer (I think this is only used as a configuration object. This is
        // one of windows' wierd quircks, which don't really make sense anyways)
        let mut buffer_description: ffi::DSBUFFERDESC = unsafe { mem::zeroed() };
        buffer_description.dwSize = mem::size_of::<ffi::DSBUFFERDESC>() as u32;
        buffer_description.dwFlags = ffi::DSBCAPS_PRIMARYBUFFER;

        let mut primary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
        let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut primary_buffer, ptr::null_mut()) };
        if result != ffi::DS_OK {
            println!("Failed to call DirectSound->CreateSoundBuffer. Error code: {}", result);
            return None;
        }
        assert!(!primary_buffer.is_null());
        let primary_buffer = unsafe { &mut *primary_buffer };

        let bytes_per_sample = mem::size_of::<SampleData>();
        let bytes_per_frame  = bytes_per_sample * CHANNELS;
        let bytes_per_second = bytes_per_frame * sample_rate as usize;
        let buffer_size      = bytes_per_frame * duration_in_frames as usize;

        let mut wave_format = ffi::WAVEFORMATEX {
            wFormatTag:      ffi::WAVE_FORMAT_PCM,
            nChannels:       CHANNELS as u16,
            nSamplesPerSec:  sample_rate,
            nAvgBytesPerSec: bytes_per_second as u32,
            nBlockAlign:     bytes_per_frame as u16,
            wBitsPerSample:  8 * bytes_per_sample as u16,
            cbSize: 0,
            .. unsafe { mem::zeroed() }
        };
        let result = unsafe { primary_buffer.SetFormat(&wave_format) };
        if result != ffi::DS_OK {
            println!("Failed call to SoundBuffer->SetFormat. Error code: {}", result);
            return None;
        }

        // Create secondary buffer (which is the buffer we actually want)
        let mut buffer_description: ffi::DSBUFFERDESC = unsafe { mem::zeroed() };
        buffer_description.dwSize = mem::size_of::<ffi::DSBUFFERDESC>() as u32;
        buffer_description.dwBufferBytes = buffer_size as u32;
        buffer_description.lpwfxFormat = &mut wave_format;
        buffer_description.dwFlags = ffi::DSBCAPS_GLOBALFOCUS;

        let mut secondary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
        let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut secondary_buffer, ptr::null_mut()) };
        if result != ffi::DS_OK {
            println!("Failed call to SoundBuffer->SetFormat. Error code: {}", result);
            return None;
        }
        assert!(!secondary_buffer.is_null());
        let secondary_buffer = unsafe { &mut *secondary_buffer };

        Some(AudioBackend {
            buffer_size,
            playing: false,
            secondary_buffer,
        })
    }

    pub fn write(
        &mut self,
        frame_counter: &mut u64,
        buffers: &[AudioBuffer],
        sounds:  &mut [Sound],
    ) {
        // Figure out where and how much to write
        let mut write_cursor = 0;
        let mut play_cursor  = 0;
        let result = unsafe { self.secondary_buffer.GetCurrentPosition(
            &mut play_cursor,
            &mut write_cursor,
        )};
        if result != ffi::DS_OK {
            println!("Failed to get current buffer position. Error code: {}", result);
            return;
        }

        let play_cursor = play_cursor as usize;

        let bytes_per_sample = mem::size_of::<SampleData>();
        let bytes_per_frame = bytes_per_sample * CHANNELS as usize;

        let our_cursor = (*frame_counter as usize * bytes_per_frame) % self.buffer_size;
        let len = {
            if play_cursor == 0 && write_cursor == 0 {
                self.buffer_size
            } else if our_cursor > play_cursor {
                (self.buffer_size - our_cursor) + play_cursor
            } else {
                play_cursor - our_cursor
            }
        };

        if len == 0 {
            return;
        }

        assert!(our_cursor < self.buffer_size);
        assert!(len <= self.buffer_size);

        // Lock secondary buffer, get write region
        let mut len1 = 0;
        let mut ptr1 = ptr::null_mut();
        let mut len2 = 0;
        let mut ptr2 = ptr::null_mut();

        let result = unsafe { self.secondary_buffer.Lock(
            our_cursor as u32, len as u32,
            &mut ptr1, &mut len1,
            &mut ptr2, &mut len2,
            0,
        )};

        if result != ffi::DS_OK {
            let result = unsafe { mem::transmute::<i32, u32>(result) };
            let msg = match result {
                0x88780096 => "Buffer lost",
                0x88780032 => "Invalid call",
                0x80070057 => "Invalid parameter",
                0x88780046 => "Priority level needed",
                _ => "Unkown error",
            };

            println!("Failed to lock secondary buffer. Error code: 0x{:x} ({})", result, msg);
            return;
        }

        assert!(len == (len1 + len2) as usize);

        // Zero out the data before we mix new sound into it
        unsafe {
            ptr::write_bytes(ptr1 as *mut u8, 0, len1 as usize);
            ptr::write_bytes(ptr2 as *mut u8, 0, len2 as usize);
        }

        // Convert to slices so we can do safe code again
        let (slice1, slice2) = unsafe {(
            slice::from_raw_parts_mut(ptr1 as *mut SampleData, len1 as usize / bytes_per_sample),
            slice::from_raw_parts_mut(ptr2 as *mut SampleData, len2 as usize / bytes_per_sample),
        )};

        // Write sound data
        let target_start_frame = *frame_counter;
        let target_mid_frame   = target_start_frame + (len1 as u64 / bytes_per_frame as u64);
        let target_end_frame   = target_start_frame + (len as u64 / bytes_per_frame as u64);

        for sound in sounds.iter_mut() {
            let ref buffer = buffers[sound.buffer];
            let sound_start_frame = sound.start_frame;
            let sound_end_frame = sound_start_frame + buffer.frames();

            let start_frame = max(sound_start_frame, target_start_frame);
            let end_frame   = min(sound_end_frame, target_end_frame);
            let mid_frame = max(min(target_mid_frame, end_frame), start_frame);

            assert_eq!(slice1.len(), (target_mid_frame - target_start_frame) as usize * CHANNELS);
            assert_eq!(slice2.len(), (target_end_frame - target_mid_frame) as usize * CHANNELS);

            if end_frame < target_start_frame {
                sound.done = true;
            }

            if start_frame < end_frame {
                let a = (start_frame - sound_start_frame) as usize * buffer.channels as usize;
                let b = (end_frame - sound_start_frame) as usize * buffer.channels as usize;
                let read_data = &buffer.data[a..b];

                let write_data_1 = if mid_frame > start_frame {
                    let a = (start_frame - target_start_frame) as usize * CHANNELS;
                    let b = (mid_frame - target_start_frame) as usize * CHANNELS;
                    &mut slice1[a..b]
                } else {
                    &mut []
                };

                let write_data_2 = if mid_frame < end_frame {
                    let a = (mid_frame - target_mid_frame) as usize * CHANNELS;
                    let b = (end_frame - target_mid_frame) as usize * CHANNELS;
                    &mut slice2[a..b]
                } else {
                    &mut []
                };

                // TODO this assumes buffer is single channel
                for frame in 0..read_data.len() {
                    let read_frame = frame*(buffer.channels as usize);
                    let write_frame = frame*CHANNELS;

                    let sample = read_data[read_frame];

                    let slot = if write_frame < write_data_1.len() {
                        &mut write_data_1[write_frame]
                    } else {
                        &mut write_data_2[write_frame - write_data_1.len()]
                    };

                    *slot = if sample > 0 {
                        slot.saturating_add(sample)
                    } else if sample == i16::min_value() {
                        slot.saturating_sub(-(sample + 1)).saturating_sub(1)
                    } else {
                        slot.saturating_sub(-sample)
                    };
                }
            }
        }
        *frame_counter = target_end_frame;

        // Unlock buffer
        let result = unsafe { self.secondary_buffer.Unlock(
            ptr1, len1, 
            ptr2, len2,
        )};
        if result != ffi::DS_OK {
            println!("Failed to unlock secondary buffer. Error code: {}", result);
        } 

        // Ensure we are playing
        if !self.playing {
            self.playing = true;
            unsafe { self.secondary_buffer.Play(0, 0, ffi::DSBPLAY_LOOPING) };
        }
    }
}

fn encode_wide(s: &str) -> Vec<u16> {
    let mut data = Vec::with_capacity(s.len() + 1);
    for wchar in s.encode_utf16() {
        data.push(wchar);
    }
    data.push(0);
    data
}

#[inline(always)]
fn min(a: u64, b: u64) -> u64 {
    if a > b { b } else { a }
}

#[inline(always)]
fn max(a: u64, b: u64) -> u64 {
    if a > b { a } else { b }
}

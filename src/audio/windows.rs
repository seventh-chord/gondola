
extern crate winapi;
extern crate kernel32;

use std::mem;
use std::slice;
use std::ptr;

use super::*;
use time::{Time, Timer};

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
    // Total size of secondary buffer, in bytes. This can't be a constant because we can't call 
    // mem::size_of::<SampleData>() at compile time
    buffer_size: usize,

    // These values are in bytes
    last_play_cursor: usize,
    play_write_cursor_gap: usize,
    cursor_granularity: usize,

    secondary_buffer: &'static mut ffi::IDirectSoundBuffer,
}

impl AudioBackend {
    pub fn initialize(window_handle: usize) -> Option<AudioBackend> {
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

        let result = unsafe { dsound.SetCooperativeLevel(window_handle as *mut _, ffi::DSSCL_PRIORITY) };
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
        let bytes_per_frame  = bytes_per_sample * OUTPUT_CHANNELS as usize;
        let bytes_per_second = bytes_per_frame * OUTPUT_SAMPLE_RATE as usize;
        let buffer_size      = bytes_per_frame * OUTPUT_BUFFER_SIZE_IN_FRAMES;

        let mut wave_format = ffi::WAVEFORMATEX {
            wFormatTag:      ffi::WAVE_FORMAT_PCM,
            nChannels:       OUTPUT_CHANNELS as u16,
            nSamplesPerSec:  OUTPUT_SAMPLE_RATE,
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
        buffer_description.dwFlags = ffi::DSBCAPS_GLOBALFOCUS | ffi::DSBCAPS_GETCURRENTPOSITION2;

        let mut secondary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
        let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut secondary_buffer, ptr::null_mut()) };
        if result != ffi::DS_OK {
            println!("Failed call to SoundBuffer->SetFormat. Error code: {}", result);
            return None;
        }
        assert!(!secondary_buffer.is_null());
        let secondary_buffer = unsafe { &mut *secondary_buffer };

        // Start playing
        unsafe { secondary_buffer.Play(0, 0, ffi::DSBPLAY_LOOPING) };

        // Compute granularity
        let mut gap_sum = 0;
        let mut jump_sum = 0;
        let mut check_count = 0;

        let mut timer = Timer::new();

        let mut last_play_cursor = 0;
        loop {
            let mut write_cursor = 0;
            let mut play_cursor  = 0;
            let result = unsafe { secondary_buffer.GetCurrentPosition(
                &mut play_cursor,
                &mut write_cursor,
            )};
            if result != ffi::DS_OK {
                println!("Failed to get current buffer position. Error code: {}", result);
                return None;
            }
            let play_cursor = play_cursor as usize;
            let write_cursor = write_cursor as usize;

            let play_cursor_jump = {
                if last_play_cursor > play_cursor {
                    play_cursor + (buffer_size - last_play_cursor)
                } else {
                    play_cursor - last_play_cursor
                }
            };
            last_play_cursor = play_cursor;

            let block_gap = {
                if write_cursor < play_cursor { 
                    write_cursor + (buffer_size - play_cursor)
                } else {
                    write_cursor - play_cursor
                }
            };

            if play_cursor_jump != 0 {
                gap_sum += block_gap;
                jump_sum += play_cursor_jump;
                check_count += 1;
            }

            let (time, _) = timer.tick();

            if check_count > 10 || time > Time::from_ms(500) {
                break;
            }
        }

        if check_count == 0 {
            println!("Unable to determine sound card latency, can not output audio");
            return None;
        }

        let play_write_cursor_gap = gap_sum / check_count;
        let cursor_granularity = jump_sum / check_count;

        println!("{} {} ", cursor_granularity, play_write_cursor_gap);
        println!("{} checks, {} seconds", check_count, timer.tick().0.as_secs_float());

        // TODO if we use 2*cursor_granularity later we have to change this
        if play_write_cursor_gap + cursor_granularity > buffer_size {
            println!(
                "Internal audio buffer is to small, given latency. Min. size is {} + {}, current \
                size is {}", 
                play_write_cursor_gap, cursor_granularity, buffer_size
            );
            return None;
        }

        Some(AudioBackend {
            buffer_size,
            last_play_cursor,
            play_write_cursor_gap,
            cursor_granularity,
            secondary_buffer,
        })
    }

    pub fn write(
        &mut self,
        frame_counter: &mut u64,
        buffers: &[AudioBuffer],
        events:  &mut [Event],
    ) {
        // The play cursor advances in chunks of ´cursor_granularity´. We can start writing
        // at `write_cursor + cursor_granularity` (to acount for uncertainty). We allways write
        // `cursor_granularity` bytes of data.

        // Get current state of playback
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
        let write_cursor = write_cursor as usize;

        let bytes_per_sample = mem::size_of::<SampleData>();
        let bytes_per_frame = bytes_per_sample * OUTPUT_CHANNELS as usize;

        let current_play_write_cursor_gap = {
            if write_cursor < play_cursor { 
                write_cursor + (self.buffer_size - play_cursor)
            } else {
                write_cursor - play_cursor
            }
        };

        let play_cursor_jump = {
            if self.last_play_cursor <= play_cursor {
                play_cursor - self.last_play_cursor
            } else {
                play_cursor + (self.buffer_size - self.last_play_cursor)
            }
        };
        self.last_play_cursor = play_cursor;

        // We did not actually jump, don't update now
        if play_cursor_jump == 0 {
            println!("Calls to `write` are to frequent");
            return;
        } else {
            println!("Nice");
        }

        // Ensure that we don't have any hiccups
        if current_play_write_cursor_gap != self.play_write_cursor_gap {
            println!(
                "Error in audio: Gap between play/write cursor changed from default {} to {}",
                self.play_write_cursor_gap, current_play_write_cursor_gap
            );
            // TODO properly handle this case, if it happens once/twice we are fine. If it happens
            // all the time, we want to back out of doing audio for a while!
        }

        if play_cursor_jump != self.cursor_granularity {
            println!(
                "Error in audio: `write` was not called at a high enough frequency, so we missed \
                a write window. Expected to jump by {}, but jumped by {}",
                self.cursor_granularity, play_cursor_jump,
            );
            // TODO Also handle this case properly
        }

        // TODO we should probably be able to run even though play_cursor_jump varies between 0 and
        // self.cursor_granularity!

        // Figure out where we want to write
        let write_start = (write_cursor + self.cursor_granularity) % self.buffer_size;
        let len = self.cursor_granularity;

        // Lock secondary buffer, get write region
        let mut len1 = 0;
        let mut ptr1 = ptr::null_mut();
        let mut len2 = 0;
        let mut ptr2 = ptr::null_mut();

        let result = unsafe { self.secondary_buffer.Lock(
            write_start as u32, len as u32,
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

        assert!(len == (len1 + len2) as usize); // Make sure we got the promissed amount of data

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

        assert_eq!(slice1.len(), (target_mid_frame - target_start_frame) as usize * OUTPUT_CHANNELS as usize);
        assert_eq!(slice2.len(), (target_end_frame - target_mid_frame) as usize   * OUTPUT_CHANNELS as usize);

        for event in events.iter_mut() {
            let ref buffer = buffers[event.buffer];

            if event.start_frame == 0 {
                event.start_frame = target_start_frame;
            }

            let event_start_frame = event.start_frame;
            let event_end_frame   = event_start_frame + buffer.frames();

            let start_frame = max(event_start_frame, target_start_frame);
            let end_frame   = min(event_end_frame, target_end_frame);
            let mid_frame   = max(min(target_mid_frame, end_frame), start_frame);

            if end_frame < target_start_frame {
                event.done = true;
            }

            if start_frame < end_frame {
                let a = (start_frame - event_start_frame) as usize * buffer.channels as usize;
                let b = (end_frame - event_start_frame) as usize   * buffer.channels as usize;
                let read_data = &buffer.data[a..b];

                let write_data_1 = if mid_frame > start_frame {
                    let a = (start_frame - target_start_frame) as usize * OUTPUT_CHANNELS as usize;
                    let b = (mid_frame - target_start_frame) as usize   * OUTPUT_CHANNELS as usize;
                    &mut slice1[a..b]
                } else {
                    &mut []
                };

                let write_data_2 = if mid_frame < end_frame {
                    let a = (mid_frame - target_mid_frame) as usize * OUTPUT_CHANNELS as usize;
                    let b = (end_frame - target_mid_frame) as usize * OUTPUT_CHANNELS as usize;
                    &mut slice2[a..b]
                } else {
                    &mut []
                };

                // TODO properly mix into channels
                for frame in 0..read_data.len() {
                    let read_frame  = frame*(buffer.channels as usize);
                    let write_frame = frame*(OUTPUT_CHANNELS as usize);

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


extern crate winapi;
extern crate kernel32;

use std::mem;
use std::slice;
use std::ptr;
//use std::ffi::CStr;

use super::*;
use time::{Time, Timer};

// We access all ffi stuff through `ffi::whatever` instead of through each apis specific
// bindings. This allows us to easily add custom stuff that is missing in bindings.
mod ffi {
    #![allow(non_camel_case_types)]

    pub(super) use super::winapi::*;
    pub(super) use super::kernel32::*;

//    pub(super) type LPDSENUMCALLBACK = Option<unsafe extern "system" fn(LPGUID, LPCSTR, LPCSTR, LPVOID) -> BOOL>;

    // Direct-sound functions
    pub(super) type DirectSoundCreate = extern "system" fn(LPGUID, *mut LPDIRECTSOUND, LPUNKNOWN) -> HRESULT;
//    pub(super) type DirectSoundEnumerate = extern "system" fn(LPDSENUMCALLBACK, LPVOID) -> HRESULT;
}

const MIN_WRITE_CHUNK_FRAMES: usize = 400;

pub(super) struct AudioBackend {
    // Total size of secondary buffer, in bytes. This can't be a constant because we can't call 
    // mem::size_of::<SampleData>() at compile time
    buffer_size: usize,

    // These values are in bytes
    last_play_cursor: usize,
    write_chunk_size: usize,
    last_write_start: Option<usize>,
    cumulative_play_cursor_jump: usize,

    secondary_buffer: &'static mut ffi::IDirectSoundBuffer,
}

impl AudioBackend {
    pub fn initialize(window_handle: usize) -> Result<AudioBackend, ()> {
        // Load library
        let lib_name = encode_wide("dsound.dll");
        let dsound_lib = unsafe { ffi::LoadLibraryW(lib_name.as_ptr()) };

        if dsound_lib.is_null() {
            println!("Could not load \"dsound.dll\"");
            // Don't panic, just run without sound
            return Err(());
        }

        /*
        let direct_sound_enumerate = {
            let name = b"DirectSoundEnumerateA\0";
            let address = unsafe { ffi::GetProcAddress(dsound_lib, name.as_ptr() as *const _) };

            if address.is_null() {
                println!("Could not load DirectSoundEnumerateA from dsound.dll");
                return Err(());
            } else {
                unsafe { mem::transmute::<_, ffi::DirectSoundEnumerate>(address) }
            }
        };

        unsafe extern "system" 
        fn callback(
            guid: ffi::LPGUID,
            description: ffi::LPCSTR,
            module: ffi::LPCSTR,
            context: ffi::LPVOID
        ) -> ffi::BOOL 
        {
            let description = CStr::from_ptr(description).to_string_lossy();
            let module      = CStr::from_ptr(module).to_string_lossy();

            println!("{:x}: \"{}\", \"{}\"", guid as usize, description, module);

            return ffi::TRUE;
        }

        direct_sound_enumerate(Some(callback), ptr::null_mut());
        */

        // Create DirectSound object
        let direct_sound_create = {
            let name = b"DirectSoundCreate\0";
            let address = unsafe { ffi::GetProcAddress(dsound_lib, name.as_ptr() as *const _) };

            if address.is_null() {
                println!("Could not load DirectSoundCreate from dsound.dll");
                return Err(());
            } else {
                unsafe { mem::transmute::<_, ffi::DirectSoundCreate>(address) }
            }
        };

        let mut dsound: ffi::LPDIRECTSOUND = ptr::null_mut();
        let result = direct_sound_create(ptr::null_mut(), &mut dsound, ptr::null_mut());
        if result != ffi::DS_OK {
            println!("Failed to create a direct sound object: {}", result);
            return Err(());
        }
        assert!(!dsound.is_null());
        let dsound = unsafe { &mut *dsound };

        let result = unsafe { dsound.SetCooperativeLevel(window_handle as *mut _, ffi::DSSCL_PRIORITY) };
        if result != ffi::DS_OK {
            println!("Failed to call DirectSound->SetCooperativeLevel. Error code: {}", result);
            return Err(());
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
            return Err(());
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
            return Err(());
        }

        // Create secondary buffer (which is the buffer we actually want)
        let mut buffer_description: ffi::DSBUFFERDESC = unsafe { mem::zeroed() };
        buffer_description.dwSize = mem::size_of::<ffi::DSBUFFERDESC>() as u32;
        buffer_description.dwBufferBytes = buffer_size as u32;
        buffer_description.lpwfxFormat = &mut wave_format;
        buffer_description.dwFlags = ffi::DSBCAPS_GLOBALFOCUS | ffi::DSBCAPS_GETCURRENTPOSITION2;

        // TODO how do we check if this flag is supported?
        buffer_description.dwFlags |= ffi::DSBCAPS_TRUEPLAYPOSITION;

        let mut secondary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
        let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut secondary_buffer, ptr::null_mut()) };
        if result != ffi::DS_OK {
            println!("Failed call to SoundBuffer->SetFormat. Error code: {}", result);
            return Err(());
        }
        assert!(!secondary_buffer.is_null());
        let secondary_buffer = unsafe { &mut *secondary_buffer };

        // Zero out secondary buffer before using it
        {
            let mut len1 = 0;
            let mut ptr1 = ptr::null_mut();
            let mut len2 = 0;
            let mut ptr2 = ptr::null_mut();
            let result = unsafe { secondary_buffer.Lock(
                0, buffer_size as u32,
                &mut ptr1, &mut len1,
                &mut ptr2, &mut len2,
                0,
            )};
            if result != ffi::DS_OK {
                println!("Failed to lock secondary buffer. Error code: {}", result);
                return Err(());
            }

            assert_eq!(len1, buffer_size as u32);
            assert_eq!(len2, 0);

            unsafe { ptr::write_bytes(ptr1 as *mut u8, 0, buffer_size) };

            let result = unsafe { secondary_buffer.Unlock(ptr1, len1, ptr2, len2)};
            if result != ffi::DS_OK {
                println!("Failed to unlock secondary buffer. Error code: {}", result);
                return Err(());
            } 
        }

        unsafe { secondary_buffer.Play(0, 0, ffi::DSBPLAY_LOOPING) };

        // If DSBCAPS_TRUEPLAYPOSITION is set, GetCursorPosition will report the frame-by-frame
        // position of the play cursor. Otherwise, the play cursor will jump in discrete chunks.
        // Regardless, the write cursor will allways jump in discrete chunks. We usually want to
        // write exactly the size between two chunks, as this will give us the lowest latency.
        let mut min_jump = usize::max_value();
        let mut check_count = 0;
        let mut timer = Timer::new();

        let mut last_play_cursor;
        let mut last_write_cursor = 0;
        loop {
            let mut write_cursor = 0;
            let mut play_cursor  = 0;
            let result = unsafe { secondary_buffer.GetCurrentPosition(
                &mut play_cursor,
                &mut write_cursor,
            )};
            if result != ffi::DS_OK {
                println!("Failed to get current buffer position. Error code: {}", result);
                return Err(());
            }
            let play_cursor = play_cursor as usize;
            let write_cursor = write_cursor as usize;

            let jump = {
                if last_write_cursor > write_cursor {
                    write_cursor + (buffer_size - last_write_cursor)
                } else {
                    write_cursor - last_write_cursor
                }
            };

            last_write_cursor = write_cursor;
            last_play_cursor = play_cursor;

            if jump > 0 {
                if jump < min_jump {
                    min_jump = jump;
                }
                check_count += 1;
            }

            let (time, _) = timer.tick();
            if check_count > 20 || time > Time::from_ms(500) {
                break;
            }
        }

        if check_count == 0 {
            println!("Unable to determine sound card latency, can not output audio");
            return Err(());
        }

        let cursor_granularity = {
            if (min_jump % bytes_per_frame) != 0 {
                ((min_jump / bytes_per_frame) + 1) * bytes_per_frame
            } else {
                min_jump
            }
        };

        let write_chunk_size = max(
            MIN_WRITE_CHUNK_FRAMES * (OUTPUT_CHANNELS as usize) * mem::size_of::<SampleData>(),
            cursor_granularity,
        );

        Ok(AudioBackend {
            buffer_size,
            last_play_cursor,
            write_chunk_size,
            last_write_start: None,
            cumulative_play_cursor_jump: 0,
            secondary_buffer,
        })
    }

    pub fn write(
        &mut self,
        frame_counter: &mut u64,
        buffers: &[AudioBuffer],
        events:  &mut [Event],
    ) -> Result<bool, ()> {
        // The play cursor advances in chunks of ´write_chunk_size´. We can start writing
        // at `write_cursor + write_chunk_size` (to acount for uncertainty). We allways write
        // `write_chunk_size` bytes of data.

        // Get current state of playback
        let mut write_cursor = 0;
        let mut play_cursor  = 0;
        let result = unsafe { self.secondary_buffer.GetCurrentPosition(
            &mut play_cursor,
            &mut write_cursor,
        )};
        if result != ffi::DS_OK {
            println!("Failed to get current buffer position. Error code: {}", result);
            return Err(());
        }
        let play_cursor = play_cursor as usize;
        let write_cursor = write_cursor as usize;

        let bytes_per_sample = mem::size_of::<SampleData>();
        let bytes_per_frame = bytes_per_sample * OUTPUT_CHANNELS as usize;

        let play_cursor_jump = {
            if self.last_play_cursor <= play_cursor {
                play_cursor - self.last_play_cursor
            } else {
                play_cursor + (self.buffer_size - self.last_play_cursor)
            }
        };
        self.last_play_cursor = play_cursor;

        // Play cursor has not moved yet, so we need to wait with writing. Maybe more events are
        // registered before we need to write.
        if play_cursor_jump <= 0 {
            return Ok(false);
        }
        self.cumulative_play_cursor_jump += play_cursor_jump;

        // Figure out where we want to write
        let write_start;
        let len = self.write_chunk_size;

        if let Some(last_write_start) = self.last_write_start {
            if self.cumulative_play_cursor_jump < self.write_chunk_size {
                return Ok(false);
            }

            let jumps = self.cumulative_play_cursor_jump / self.write_chunk_size;
            if jumps > 1 {
                println!(
                    "Calls to `backend::write` were to infrequent, the write cursor has overrun a \
                    region we have not yet written to. It has jumped {} write chunks, but should \
                    at most ever jump 1 chunk. In total, it has jumped {} bytes",
                    jumps,
                    self.cumulative_play_cursor_jump,
                );

                // TODO if this happens repeatedly, we really just have to give up playing sound!
                // We probably should track how often this happens, and let the audio system
                // decide to give up playing based on what we track!
            }

            self.cumulative_play_cursor_jump -= jumps*self.write_chunk_size;
            write_start = (last_write_start + jumps*self.write_chunk_size) % self.buffer_size;
        } else {
            write_start = (write_cursor + self.write_chunk_size) % self.buffer_size;
            self.cumulative_play_cursor_jump = 0;
        }

        self.last_write_start = Some(write_start);

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
            return Err(());
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

        return Ok(true);
    }

    /// The time between each consecutive write. If one write occured at t0, the next call to write
    /// must be somewhere between `t0 + interval` and `t0 + 2*interval`. The data must be written by
    /// `t0 + 2*interval`
    pub fn write_interval(&self) -> Time {
        let bytes_per_sample = mem::size_of::<SampleData>();
        let bytes_per_frame = bytes_per_sample * OUTPUT_CHANNELS as usize;
        let frames_per_write = (self.write_chunk_size / bytes_per_frame) as u64;

        Time(frames_per_write*Time::NANOSECONDS_PER_SECOND/(OUTPUT_SAMPLE_RATE as u64))
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
fn min<T: Ord + Copy>(a: T, b: T) -> T {
    if a > b { b } else { a }
}

#[inline(always)]
fn max<T: Ord + Copy>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

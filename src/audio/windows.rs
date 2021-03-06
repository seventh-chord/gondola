
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

const BUFFER_SIZE_IN_FRAMES: usize = 2 * (OUTPUT_SAMPLE_RATE as usize);
const MIN_WRITE_CHUNK_SIZE_IN_FRAMES: usize = 240;

pub(super) struct AudioBackend {
    // Total size of secondary buffer, in bytes. This can't be a constant because we can't call 
    // mem::size_of::<SampleData>() at compile time
    buffer_size: usize,

    // These values are in bytes
    last_play_cursor: usize,
    write_chunk_size: usize,
    last_write: Option<(usize, usize)>, // Start and length
    cumulative_play_cursor_jump: usize,

    secondary_buffer: &'static mut ffi::IDirectSoundBuffer,
}

impl AudioBackend {
    pub fn initialize(window_handle: usize) -> Result<AudioBackend, AudioError> {
        // Load library
        let library_name = b"dsound.dll\0";
        let dsound_lib = unsafe { ffi::LoadLibraryA(library_name.as_ptr() as *const i8) };

        if dsound_lib.is_null() {
            let message = "Could not load library \"dsound.dll\"".to_owned();
            return Err(AudioError::Other { message });
        }

        // Create DirectSound object
        let direct_sound_create = {
            let name = b"DirectSoundCreate\0";
            let address = unsafe { ffi::GetProcAddress(dsound_lib, name.as_ptr() as *const _) };

            if address.is_null() {
                let message = "No `DirectSoundCreate` in \"dsound.dll\"".to_owned();
                return Err(AudioError::Other { message });
            } else {
                unsafe { mem::transmute::<_, ffi::DirectSoundCreate>(address) }
            }
        };

        let mut dsound: ffi::LPDIRECTSOUND = ptr::null_mut();
        let result = direct_sound_create(ptr::null_mut(), &mut dsound, ptr::null_mut());
        if result != ffi::DS_OK {
            return Err(AudioError::BadReturn {
                function_name: "DirectSoundCreate".to_owned().to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
        }
        assert!(!dsound.is_null());
        let dsound = unsafe { &mut *dsound };

        let result = unsafe { dsound.SetCooperativeLevel(window_handle as *mut _, ffi::DSSCL_PRIORITY) };
        if result != ffi::DS_OK {
            return Err(AudioError::BadReturn { 
                function_name: "DirectSound->SetCooperativeLevel".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
        }

        // Create primary buffer (I think this is only used as a configuration object. This is
        // one of windows' wierd quircks, which don't really make sense anyways)
        let mut buffer_description: ffi::DSBUFFERDESC = unsafe { mem::zeroed() };
        buffer_description.dwSize = mem::size_of::<ffi::DSBUFFERDESC>() as u32;
        buffer_description.dwFlags = ffi::DSBCAPS_PRIMARYBUFFER;

        let mut primary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
        let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut primary_buffer, ptr::null_mut()) };
        if result != ffi::DS_OK {
            return Err(AudioError::BadReturn { 
                function_name: "DirectSound->CreateSoundBuffer".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
        }
        assert!(!primary_buffer.is_null());
        let primary_buffer = unsafe { &mut *primary_buffer };

        let bytes_per_sample = mem::size_of::<SampleData>();
        let bytes_per_frame  = bytes_per_sample * OUTPUT_CHANNELS as usize;
        let bytes_per_second = bytes_per_frame * OUTPUT_SAMPLE_RATE as usize;
        let buffer_size      = bytes_per_frame * BUFFER_SIZE_IN_FRAMES;

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
            return Err(AudioError::BadReturn { 
                function_name: "DirectSoundBuffer->SetFormat".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
        }

        // Create secondary buffer (which is the buffer we actually want)
        let mut buffer_description: ffi::DSBUFFERDESC = unsafe { mem::zeroed() };
        buffer_description.dwSize = mem::size_of::<ffi::DSBUFFERDESC>() as u32;
        buffer_description.dwBufferBytes = buffer_size as u32;
        buffer_description.lpwfxFormat = &mut wave_format;
        buffer_description.dwFlags = ffi::DSBCAPS_GLOBALFOCUS | ffi::DSBCAPS_GETCURRENTPOSITION2;

        // NB this will only work on vista. I don't know if this causes an error, or is just
        // ignored on XP. The sound system currently works without this flag though!
        buffer_description.dwFlags |= ffi::DSBCAPS_TRUEPLAYPOSITION;

        let mut secondary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
        let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut secondary_buffer, ptr::null_mut()) };
        if result != ffi::DS_OK {
            return Err(AudioError::BadReturn {
                function_name: "DirectSound->CreateSoundBuffer".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
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
                return Err(AudioError::BadReturn {
                    function_name: "DirectSoundBuffer->Lock".to_owned(),
                    error_code: result as i64,
                    line: line!(),
                    file: file!(), 
                });
            }

            assert_eq!(len1, buffer_size as u32);
            assert_eq!(len2, 0);

            unsafe { ptr::write_bytes(ptr1 as *mut u8, 0, buffer_size) };

            let result = unsafe { secondary_buffer.Unlock(ptr1, len1, ptr2, len2)};
            if result != ffi::DS_OK {
                return Err(AudioError::BadReturn {
                    function_name: "DirectSoundBuffer->Unlock".to_owned(),
                    error_code: result as i64,
                    line: line!(),
                    file: file!(), 
                });
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
                return Err(AudioError::BadReturn {
                    function_name: "DirectSoundBuffer->GetCurrentPosition".to_owned(),
                    error_code: result as i64,
                    line: line!(),
                    file: file!(), 
                });
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
            let message = "Unable to determine sound card commit chunk size".to_owned();
            return Err(AudioError::Other { message });
        }

        let cursor_granularity = {
            if (min_jump % bytes_per_frame) != 0 {
                ((min_jump / bytes_per_frame) + 1) * bytes_per_frame
            } else {
                min_jump
            }
        };

        let min_write_chunk_size = MIN_WRITE_CHUNK_SIZE_IN_FRAMES * bytes_per_frame;
        let write_chunk_size = Ord::max(min_write_chunk_size, cursor_granularity);

        Ok(AudioBackend {
            buffer_size,
            last_play_cursor,
            write_chunk_size,
            last_write: None,
            cumulative_play_cursor_jump: 0,
            secondary_buffer,
        })
    }

    pub fn write<F>(
        &mut self,
        frame_counter: &mut u64,
        mut mix_callback: F,
    ) -> Result<bool, AudioError> 
      where F: FnMut(u64, &mut [SampleData]),
    {
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
            return Err(AudioError::BadReturn {
                function_name: "DirectSoundBuffer->GetCurrentPosition".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
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
        let mut write_start;
        let write_len;

        if let Some((last_write_start, last_write_chunks)) = self.last_write {
            // Number of whole chunks we have advanced
            let jumps = self.cumulative_play_cursor_jump / self.write_chunk_size;
            if jumps < 1 {
                return Ok(false);
            }

            self.cumulative_play_cursor_jump -= jumps*self.write_chunk_size;

            write_start = (last_write_start + last_write_chunks) % self.buffer_size;
            write_len   = jumps*self.write_chunk_size;
        } else {
            self.cumulative_play_cursor_jump = 0;

            write_start = (write_cursor + self.write_chunk_size) % self.buffer_size;
            write_len   = self.write_chunk_size;
        }

        // NB (Morten, 09.10.17)
        // This relys on write_start not falling so far behind that it looks like its ahead
        // again, which is a real issue with ring buffers. Currently, the ring buffer is two
        // seconds long, so it is unlikely that this happens, as it would mean we have essentially
        // stalled for 1 second somewhere in the audio thread. 
        let write_start_to_write_cursor: isize = {
            let distance = write_start as isize - write_cursor as isize;
            let size = self.buffer_size as isize;

            if distance < -size/2 {
                size + distance
            } else if distance > size/2 {
                distance - size
            } else {
                distance
            }
        };

        if write_start_to_write_cursor < 0 {
            let write_cursor_to_write_start = (-write_start_to_write_cursor) as usize;
            // The `-1` `+1` stuff rounds integer division up instead of down
            let chunks_behind = (write_cursor_to_write_start - 1)/self.write_chunk_size + 1;

            println!(
                "Calls to `backend::write` were to infrequent, the write cursor has overrun 
                a region we were going to write to. We are {} chunks behind!.",
                chunks_behind,
            );

            write_start = (write_start + chunks_behind*self.write_chunk_size) % self.buffer_size;
            // Maybe modify write_len?

            // TODO if this happens repeatedly, we really just have to give up playing sound!
            // We probably should track how often this happens, and let the audio system
            // decide to give up playing based on what we track!
        }

        self.last_write = Some((write_start, write_len));

        // Lock secondary buffer, get write region
        let mut len1 = 0;
        let mut ptr1 = ptr::null_mut();
        let mut len2 = 0;
        let mut ptr2 = ptr::null_mut();

        let result = unsafe { self.secondary_buffer.Lock(
            write_start as u32, write_len as u32,
            &mut ptr1, &mut len1,
            &mut ptr2, &mut len2,
            0,
        )};
        if result != ffi::DS_OK {
            return Err(AudioError::BadReturn {
                function_name: "DirectSoundBuffer->Lock".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
        }

        assert!(write_len == (len1 + len2) as usize); // Make sure we got the promissed amount of data

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
        let target_end_frame   = target_start_frame + (write_len as u64 / bytes_per_frame as u64);

        assert_eq!(slice1.len(), (target_mid_frame - target_start_frame) as usize * OUTPUT_CHANNELS as usize);
        assert_eq!(slice2.len(), (target_end_frame - target_mid_frame) as usize   * OUTPUT_CHANNELS as usize);

        if !slice1.is_empty() {
            mix_callback(target_start_frame, slice1);
        } 
        if !slice2.is_empty() {
            mix_callback(target_mid_frame, slice2);
        }

        *frame_counter = target_end_frame;

        // Unlock buffer
        let result = unsafe { self.secondary_buffer.Unlock(
            ptr1, len1, 
            ptr2, len2,
        )};
        if result != ffi::DS_OK {
            return Err(AudioError::BadReturn {
                function_name: "DirectSoundBuffer->Unlock".to_owned(),
                error_code: result as i64,
                line: line!(),
                file: file!(), 
            });
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

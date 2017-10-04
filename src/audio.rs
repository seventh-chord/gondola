
//! Experimental: custom audio stuff

use window::Window;

pub trait AudioCommon {
    fn initialize(window: &Window) -> Option<Audio>;
}

#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    extern crate winapi;
    extern crate kernel32;

    use std::mem;
    use std::ptr;

    // We access all ffi stuff through `ffi::whatever` instead of through each apis specific
    // bindings. This allows us to easily add custom stuff that is missing in bindings.
    mod ffi {
        #![allow(non_camel_case_types)]

        pub(super) use super::winapi::*;
        pub(super) use super::kernel32::*;

        // Direct-sound functions
        pub(super) type DirectSoundCreate = extern "system" fn(LPGUID, *mut LPDIRECTSOUND, LPUNKNOWN) -> HRESULT;
    }

    pub struct Audio {
    }

    impl AudioCommon for Audio {
        fn initialize(window: &Window) -> Option<Audio> {
            let sample_rate: u32 = 44100u32; // TODO pass this in
            let buffer_duration_seconds: u32 = 2; // TODO pass this in
            let channels: u32 = 2;
            let bits_per_sample: u32 = 16;

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

            let mut wave_format = ffi::WAVEFORMATEX {
                wFormatTag:      ffi::WAVE_FORMAT_PCM,
                nChannels:       channels as u16,
                nSamplesPerSec:  sample_rate,
                nAvgBytesPerSec: ((bits_per_sample/8) * channels) as u32 * sample_rate,
                nBlockAlign:     ((bits_per_sample/8) * channels) as u16,
                wBitsPerSample:  bits_per_sample as u16,
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
            buffer_description.dwBufferBytes = sample_rate*buffer_duration_seconds*(bits_per_sample/8);
            buffer_description.lpwfxFormat = &mut wave_format;

            let mut secondary_buffer: ffi::LPDIRECTSOUNDBUFFER = ptr::null_mut();
            let result = unsafe { dsound.CreateSoundBuffer(&buffer_description, &mut secondary_buffer, ptr::null_mut()) };
            if result != ffi::DS_OK {
                println!("Failed call to SoundBuffer->SetFormat. Error code: {}", result);
                return None;
            }

            println!("Succesfully initialized audio");

            Some(Audio {
            })
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
}

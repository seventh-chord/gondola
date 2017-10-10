
// NB (Morten, 09.10.17)
// Currently, we assume SampleData to be i16!
// See SND_PCM_FORMAT_S16_LE

extern crate alsa_sys as alsa;

use std::mem;
use std::ffi::CStr;

use super::*;
use time::Time;

const MAX_WRITE_FRAMES: u64 = 1024;

pub(super) struct AudioBackend {
    pcm_handle: *mut alsa::snd_pcm_t,
    write_buffer: Vec<i16>,
    total_frames: u64,
}

impl AudioBackend {
    pub fn initialize() -> Result<AudioBackend, InitializationError> {
        let mut pcm_handle = ptr::null_mut();
        let mut write_buffer = Vec::new();
        let total_frames;

        unsafe {
            let device_name = b"default\0";

            let result = alsa::snd_pcm_open(
                &mut pcm_handle,
                device_name.as_ptr() as *const i8,
                alsa::SND_PCM_STREAM_PLAYBACK, 
                0
            );
            if result < 0 {
                println!("snd_pcm_open failed: {}", result);
                return Err(());
            }

            // Configure "hardware" stuff
            let mut hardware = ptr::null_mut();
            let result = alsa::snd_pcm_hw_params_malloc(&mut hardware);
            if result < 0 {
                println!("snd_pcm_hw_params_malloc failed: {}", result);
                return Err(());
            }
            assert!(!hardware.is_null());

            let result = alsa::snd_pcm_hw_params_any(pcm_handle, hardware);
            if result < 0 {
                println!("snd_pcm_hw_params_any failed: {}", result);
                return Err(());
            }

            let access = alsa::SND_PCM_ACCESS_RW_INTERLEAVED;
            let format = if cfg!(target_endian = "big") {
                alsa::SND_PCM_FORMAT_S16_BE
            } else {
                alsa::SND_PCM_FORMAT_S16_LE
            };
            let channels = OUTPUT_CHANNELS;
            let mut sample_rate = OUTPUT_SAMPLE_RATE;

            alsa::snd_pcm_hw_params_set_access(pcm_handle, hardware, access);
            alsa::snd_pcm_hw_params_set_format(pcm_handle, hardware, format);
            alsa::snd_pcm_hw_params_set_channels(pcm_handle, hardware, channels);
            alsa::snd_pcm_hw_params_set_rate_near(pcm_handle, hardware, &mut sample_rate, ptr::null_mut());

            let result = alsa::snd_pcm_hw_params(pcm_handle, hardware);
            if result < 0 {
                println!("snd_pcm_hw_params failed: {}", result);
                return Err(());
            }

            alsa::snd_pcm_hw_params_free(hardware);

            // Configure "software" stuff
            let mut software = ptr::null_mut();
            let result = alsa::snd_pcm_sw_params_malloc(&mut software);
            if result < 0 {
                println!("snd_pcm_sw_params_malloc failed: {}", result);
                return Err(());
            }
            assert!(!software.is_null());

            let result = alsa::snd_pcm_sw_params_current(pcm_handle, software);
            if result < 0 {
                println!("snd_pcm_sw_params_current failed: {}", result);
                return Err(());
            }

            alsa::snd_pcm_sw_params_set_avail_min(pcm_handle, software, MAX_WRITE_FRAMES);
            alsa::snd_pcm_sw_params_set_start_threshold(pcm_handle, software, 0);

            let result = alsa::snd_pcm_sw_params(pcm_handle, software);
            if result < 0 {
                println!("snd_pcm_sw_params failed: {}", result);
                return Err(());
            }

            alsa::snd_pcm_sw_params_free(software);

            total_frames = alsa::snd_pcm_avail(pcm_handle) as u64;

            // Prepare for playing
            let result = alsa::snd_pcm_prepare(pcm_handle);
            if result < 0 {
                println!("snd_pcm_prepare failed: {}", result);
                return Err(());
            } 

            // Write some bytes at the start to prevent buffer underruns
            let samples = MAX_WRITE_FRAMES as usize * OUTPUT_CHANNELS as usize;
            write_buffer.reserve(samples);
            ptr::write_bytes(write_buffer.as_mut_ptr(), 0, samples);

            let result = alsa::snd_pcm_writei(
                pcm_handle,
                write_buffer.as_ptr() as *const _,
                MAX_WRITE_FRAMES,
            );
            if result < 0 {
                println!("snd_pcm_writei failed: {}", result);
                return Err(());
            }
        }

        Ok(AudioBackend {
            pcm_handle,
            write_buffer,
            total_frames,
        })
    }

    pub fn write<F>(
        &mut self,
        frame_counter: &mut u64,
        mut mix_callback: F,
    ) -> Result<bool, ()> 
      where F: FnMut(u64, &mut [SampleData]),
    {
        // ALSA will request enough frames to fill up the entire ring buffer,
        // we only want to write a few frames ahead to keep latency low.

        let available_frames;

        unsafe {
            let result = alsa::snd_pcm_avail_update(self.pcm_handle);
            if result == -32 {
                // We did not provide data fast enough, recover
                let recover_result = alsa::snd_pcm_recover(self.pcm_handle, -32, 1);
                if recover_result < 0 {
                    println!("Underrun detected, could not recover");
                    return Err(()); // We are probably fucked
                } else {

                    // Try again
                    let retry_result = alsa::snd_pcm_avail_update(self.pcm_handle);
                    if retry_result < 0 {
                        println!("Underrun detected, recovered but it did not help");
                        return Err(());
                    } else {
                        println!("Underrun detected and fixed");
                        available_frames = retry_result as u64;
                    }
                }

            } else if result < 0 {
                println!("snd_pcm_avail_delay failed: {}", result);
                return Err(());
            } else {
                available_frames = result as u64;
            }
        }

        if available_frames <= 0 {
            // We somehow managed to fill up the entire ring buffer, this is sort of bad
            return Ok(false);
        }

        let unplayed_frames = self.total_frames - available_frames;
        if unplayed_frames > 2*MAX_WRITE_FRAMES {
            return Ok(false);
        }

        let write_frames = if unplayed_frames < MAX_WRITE_FRAMES {
            2*MAX_WRITE_FRAMES - unplayed_frames
        } else {
            MAX_WRITE_FRAMES
        };
        let samples = write_frames as usize * OUTPUT_CHANNELS as usize;

        self.write_buffer.clear();
        self.write_buffer.reserve(samples);
        unsafe {
            self.write_buffer.set_len(samples);
            ptr::write_bytes(self.write_buffer.as_mut_ptr(), 0, samples);
        }

        mix_callback(*frame_counter, &mut self.write_buffer);
        *frame_counter += write_frames;

        unsafe {
            // TODO we might also get a underrun here, we probably can recover from that as well!
            let result = alsa::snd_pcm_writei(
                self.pcm_handle,
                self.write_buffer.as_ptr() as *const _,
                write_frames,
            );
            if result < 0 {
                println!("snd_pcm_writei failed: {}", result);
                return Err(());
            }
        }

        return Ok(true); // We wrote some data
    }

    pub fn write_interval(&self) -> Time {
        Time((MAX_WRITE_FRAMES as u64 * Time::NANOSECONDS_PER_SECOND) / OUTPUT_SAMPLE_RATE as u64)
    }
}

impl Drop for AudioBackend {
    fn drop(&mut self) {
        unsafe {
            alsa::snd_pcm_close(self.pcm_handle);
        }
    }
}

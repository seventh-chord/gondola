
extern crate alsa_sys as alsa;

use std::ffi::CStr;

use super::*;
use time::Time;

pub(super) struct AudioBackend {
}

impl AudioBackend {
    pub fn initialize() -> Result<AudioBackend, ()> {
        let mut pcm_handle = ptr::null_mut();
        let mut hardware_parameters = ptr::null_mut();

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

            let result = alsa::snd_pcm_hw_params_malloc(&mut hardware_parameters);
            if result < 0 {
                println!("snd_pcm_hw_params_malloc failed: {}", result);
                return Err(());
            }

            // Configure "hardware" stuff
            let result = alsa::snd_pcm_hw_params_any(pcm_handle, hardware_parameters);
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

            alsa::snd_pcm_hw_params_set_access(pcm_handle, hardware_parameters, access);
            alsa::snd_pcm_hw_params_set_format(pcm_handle, hardware_parameters, format);
            alsa::snd_pcm_hw_params_set_channels(pcm_handle, hardware_parameters, channels);
            alsa::snd_pcm_hw_params_set_rate_near(pcm_handle, hardware_parameters, &mut sample_rate, ptr::null_mut());

            let result = alsa::snd_pcm_hw_params(pcm_handle, hardware_parameters);
            if result < 0 {
                println!("snd_pcm_hw_params failed: {}", result);
                return Err(());
            }

            alsa::snd_pcm_hw_params_free(hardware_parameters);

            // Play I guess
            let result = alsa::snd_pcm_prepare(pcm_handle);
            if result < 0 {
                println!("snd_pcm_prepare failed: {}", result);
                return Err(());
            }

            let mut data = Vec::with_capacity(48000 * 2);
            for i in 0..48000 {
                let t = (i as f32 / 48000.0) * 110.0;
                let v = (t * 2.0 * 3.1415).sin() * (i16::max_value() as f32);

                data.push(v as i16);
                data.push(v as i16);
            }

            for _ in 0..3 {
                println!("write");
                let result = alsa::snd_pcm_writei(
                    pcm_handle,
                    data.as_ptr() as *const _,
                    data.len() as u64
                );
                if result < 0 {
                    println!("snd_pcm_writei failed: {}", result);
                    break;
                }
            }

            ::std::thread::sleep(Time::from_secs(3).into());

            println!("Heyo");
            alsa::snd_pcm_close(pcm_handle);
        }

        /*
        snd_smixer_xx();
        snd_pcm_delay(); // Synchronization
        snd_pcm_update_avail(); // Playback/capture fill level
        snd_pcm_recover(); // Recover from errors (what?)
        // use largest possible buffer size! (We want this anyways)
        snd_pcm_rewind(); // If we need to react to user input quickly (what?)
        */

        Ok(AudioBackend {
        })
    }

    pub fn write<F>(
        &mut self,
        frame_counter: &mut u64,
        mut mix_callback: F,
    ) -> Result<bool, ()> 
      where F: FnMut(u64, &mut [SampleData]),
    {
        return Ok(true); // Write was succesfull
    }

    pub fn write_interval(&self) -> Time {
        Time::from_ms(5) // TODO
    }
}

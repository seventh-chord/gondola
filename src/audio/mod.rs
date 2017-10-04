
//! Experimental: custom audio stuff

// Note (Morten, 04.10.17)
// A sample is a single i16 (Or whatever `SampleData` is)
// A frame is one i16 per channel

// TODO fix error handling, custom error types!

use std::f32::consts::PI;

use window::Window;

// Different platforms
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

pub mod wav;

const CHANNELS: usize = 2;
type SampleData = i16;

pub struct AudioSystem {
    backend: AudioBackend,
    frame_counter: u64,
}

impl AudioSystem {
    pub fn initialize(window: &Window) -> Option<AudioSystem> {
        let backend_settings = BackendSettings {
            sample_rate: 44100,
            duration_in_frames: 44100*2,
        };

        let backend = match AudioBackend::initialize(window, backend_settings) {
            Some(b) => b,
            None => {
                return None;
            },
        };

        Some(AudioSystem {
            backend,
            frame_counter: 0,
        })
    }

    pub fn tick(&mut self) {
        self.backend.write_wave(&mut self.frame_counter);
    }
}

#[derive(Copy, Clone)]
struct BackendSettings {
    sample_rate: u32,
    duration_in_frames: u32,
}

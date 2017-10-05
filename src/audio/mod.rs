
//! Experimental: custom audio stuff

// NB (Morten, 04.10.17)
// A sample is a single i16 (Or whatever `SampleData` is)
// A frame is one i16 per channel

// TODO fix error handling, custom error types!

use window::Window;
use time::Time;

// Different platforms
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

pub mod wav;

const CHANNELS: usize = 2; // TODO channels is u8 everywhere else!
type SampleData = i16;

pub struct AudioSystem {
    sample_rate: u32,
    backend: AudioBackend,
    frame_counter: u64,

    pub buffers: Vec<AudioBuffer>,
    sounds: Vec<Sound>,
}

impl AudioSystem {
    pub fn initialize(window: &Window) -> Option<AudioSystem> {
        let sample_rate = 48000; // TODO try chaning this back to 44100
        let buffer_duration_in_frames = sample_rate / 8; // TODO should be 2*sample_rate for two seconds

        let backend = match AudioBackend::initialize(window, sample_rate, buffer_duration_in_frames) {
            Some(b) => b,
            None => {
                return None;
            },
        };

        Some(AudioSystem {
            backend,
            sample_rate,
            frame_counter: 0,
            buffers: Vec::with_capacity(30),
            sounds:  Vec::with_capacity(30),
        })
    }

    pub fn tick(&mut self) {
        self.backend.write(&mut self.frame_counter, &self.buffers, &mut self.sounds);

        // Remove sounds when they are done playing
        let mut i = 0;
        while i < self.sounds.len() {
            if self.sounds[i].done {
                self.sounds.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn play(&mut self, buffer: usize) {
        self.sounds.push(Sound {
            start_frame: self.frame_counter + (self.sample_rate/30) as u64,
            done: false,
            buffer,
        });
    }
}

#[derive(Clone)]
pub struct AudioBuffer {
    pub channels: u8,
    pub sample_rate: u32,
    pub data: Vec<i16>,
}

impl AudioBuffer {
    pub fn duration(&self) -> Time {
        let frames = self.frames();
        let frequency = self.sample_rate as u64;

        Time((frames*Time::NANOSECONDS_PER_SECOND) / frequency)
    }

    #[inline(always)]
    pub fn frames(&self) -> u64 {
        self.data.len() as u64 / self.channels as u64
    }
}

// TODO `Sound` is probably a confusing name
pub struct Sound {
    pub start_frame: u64,
    pub done: bool,
    pub buffer: usize,
}


//! Experimental: custom audio stuff

// NB (Morten, 04.10.17)
// A sample is a single i16 (Or whatever `SampleData` is)
// A frame is one i16 per channel

use window::Window;
use time::Time;

// Different platforms
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

pub mod wav;

// TODO fix error handling, custom error types!

// TODO Change sample rate back to 44100, implement resampling for sound buffers
// We don't neccesarily have to output at 44.1kHz in the end, but this would be
// an easy way to implement and test resampling

// TODO OUTPUT_BUFFER_SIZE_IN_FRAMES should be two seconds or something

const OUTPUT_CHANNELS: u32 = 2;
const OUTPUT_SAMPLE_RATE: u32 = 48000;
const OUTPUT_BUFFER_SIZE_IN_FRAMES: usize = 10000;
type SampleData = i16;

pub struct AudioSystem {
    backend: AudioBackend,
    frame_counter: u64,

    pub buffers: Vec<AudioBuffer>,
    sounds: Vec<Sound>,
}

impl AudioSystem {
    pub fn initialize(window: &Window) -> Option<AudioSystem> {
        let backend = match AudioBackend::initialize(window ) {
            Some(b) => b,
            None => {
                return None;
            },
        };

        Some(AudioSystem {
            backend,
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
            start_frame: self.frame_counter + (OUTPUT_SAMPLE_RATE/30) as u64, // TODO Figoure out how to actually do audio-video sync
            done: false,
            buffer,
        });
    }
}

#[derive(Clone)]
pub struct AudioBuffer {
    pub channels: u32,
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

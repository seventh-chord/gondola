
//! Experimental: custom audio stuff

// NB (Morten, 04.10.17)
// Regarding terminology
// A "sample" is a single i16 (Or whatever `SampleData` is): i16
// A "frame" is one i16 per channel:  (left, right): (i16, i16)

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

// TODO Multithreading is pretty much needed so we don't miss our write windows, at least with
// direct sound!

const OUTPUT_CHANNELS: u32 = 2;
const OUTPUT_SAMPLE_RATE: u32 = 48000;
const OUTPUT_BUFFER_SIZE_IN_FRAMES: usize = 2*(OUTPUT_SAMPLE_RATE as usize);
type SampleData = i16;

pub struct AudioSystem {
    backend: AudioBackend,
    frame_counter: u64,

    pub buffers: Vec<AudioBuffer>,
    events: Vec<Event>,
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
            events:  Vec::with_capacity(30),
        })
    }

    pub fn tick(&mut self) {
        self.backend.write(&mut self.frame_counter, &self.buffers, &mut self.events);

        // Remove events when they are done playing
        let mut i = 0;
        while i < self.events.len() {
            if self.events[i].done {
                self.events.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn play(&mut self, buffer: usize) {
        self.events.push(Event {
            start_frame: 0,
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

pub struct Event {
    pub start_frame: u64, // Set internally when the event is actually started
    pub done: bool,
    pub buffer: usize,
}


//! Experimental: custom audio stuff

// NB (Morten, 04.10.17)
// Regarding terminology
// A "sample" is a single i16 (Or whatever `SampleData` is): i16
// A "frame" is one i16 per channel:  (left, right): (i16, i16)

// TODO fix error handling, custom error types!

// TODO Change sample rate back to 44100, implement resampling for sound buffers
// We don't neccesarily have to output at 44.1kHz in the end, but this would be
// an easy way to implement and test resampling

use std::thread;
use std::sync::{Arc, Mutex};

use window::Window;
use time::{Time, Timer};

// Different platforms
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

pub mod wav;

const OUTPUT_CHANNELS: u32 = 2;
const OUTPUT_SAMPLE_RATE: u32 = 48000;
const OUTPUT_BUFFER_SIZE_IN_FRAMES: usize = 2*(OUTPUT_SAMPLE_RATE as usize);
type SampleData = i16;

pub struct AudioSystem {
    new_buffers: Vec<AudioBuffer>,
    new_events:  Vec<Event>,
    internal_data: Arc<Mutex<InternalData>>,
}

// Has to be shared between threads!
struct InternalData {
    buffers: Vec<AudioBuffer>,
    events:  Vec<Event>,
}

impl AudioSystem {
    pub fn initialize(window: &Window) -> AudioSystem {
        let internal_data = InternalData {
            buffers: Vec::with_capacity(100),
            events:  Vec::with_capacity(100),
        };

        let mutex = Mutex::new(internal_data);
        let arc = Arc::new(mutex);
        let weak = Arc::downgrade(&arc);

        #[cfg(target_os = "windows")]
        let window_handle = window.window_handle() as usize; // Stupid hack

        thread::spawn(move || {
            // Initialize backend
            #[cfg(target_os = "windows")]
            let backend = AudioBackend::initialize(window_handle);
            #[cfg(not(target_os = "windows"))]
            let backend = AudioBackend::initialize();

            let mut backend = match backend {
                Some(b) => b,
                None => {
                    // TODO handle errors!
                    return;
                },
            };

            let mut frame_counter = 0;
            let mut timer = Timer::new();

            loop {
                let mutex = match weak.upgrade() {
                    Some(m) => m,
                    None => {
                        // This means `AudioSystem`, which has the strong `Arc` was dropped
                        return;
                    },
                };

                let start_time = timer.tick().0;
                {
                    let internal_data = &mut *mutex.lock().unwrap();

                    backend.write(&mut frame_counter, &internal_data.buffers, &mut internal_data.events);

                    println!("{} events", internal_data.events.len());

                    // Remove events when they are done playing
                    let mut i = 0;
                    while i < internal_data.events.len() {
                        if internal_data.events[i].done {
                            internal_data.events.swap_remove(i);
                        } else {
                            i += 1;
                        }
                    }
                }
                let end_time = timer.tick().0;

                println!("Write took {}ms", (end_time - start_time).as_ms());

                // TODO do we actually need to sleep? For how long?
//                thread::sleep(Time::from_ms(1).into()); 
            }
        });

        AudioSystem {
            new_buffers: Vec::with_capacity(30),
            new_events:  Vec::with_capacity(30),
            internal_data: arc,
        }
    }

    pub fn tick(&mut self) {
        if self.new_events.is_empty() && self.new_buffers.is_empty() {
            return;
        }

        let internal_data = &mut *self.internal_data.lock().unwrap();

        // Add new events and buffers
        for new_event in self.new_events.drain(..) {
            internal_data.events.push(new_event);
        }

        for new_buffer in self.new_buffers.drain(..) {
            internal_data.buffers.push(new_buffer);
        }
    }

    pub fn play(&mut self, buffer: usize) {
        self.new_events.push(Event {
            start_frame: 0,
            done: false,
            buffer,
        });
    }

    pub fn add_buffer(&mut self, buffer: AudioBuffer) {
        self.new_buffers.push(buffer);
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

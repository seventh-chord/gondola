
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
use std::sync::mpsc;

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

type BufferHandle = usize;

pub struct AudioSystem {
    next_buffer_handle: BufferHandle,
    broken:   bool,
    receiver: mpsc::Receiver<MessageFromAudioThread>,
    sender:   mpsc::Sender<MessageToAudioThread>,
}

enum MessageToAudioThread {
    NewEvent { event: Event },
    AddBuffer { buffer: AudioBuffer },
}

enum MessageFromAudioThread {
    CriticalError,
}

impl AudioSystem {
    pub fn initialize(window: &Window) -> AudioSystem {
        // TODO Remove the stupid hack!
        #[cfg(target_os = "windows")]
        let window_handle = window.window_handle() as usize; // Stupid hack

        let (thread_sender, receiver) = mpsc::channel();
        let (sender, thread_receiver) = mpsc::channel();

        thread::spawn(move || {
            // Initialize backend
            #[cfg(target_os = "windows")]
            let backend = AudioBackend::initialize(window_handle);
            #[cfg(not(target_os = "windows"))]
            let backend = AudioBackend::initialize();

            let mut backend = match backend {
                Ok(b) => b,
                Err(()) => {
                    let _ = thread_sender.send(MessageFromAudioThread::CriticalError);
                    return;
                },
            };

            let mut frame_counter = 0;
            let mut timer = Timer::new();

            let mut buffers = Vec::with_capacity(100);
            let mut events  = Vec::with_capacity(100);

            let mut last_write = Time::ZERO;
            let mut average_write_time = Time::ZERO;
            let mut total_write_time = Time::ZERO;
            let mut write_count = 0;

            loop {
                let mut did_write = false;

                let start = timer.tick().0;

                // Actually update audio output
                let write_result = backend.write(&mut frame_counter, &buffers, &mut events);
                match write_result {
                    Ok(wrote) => {
                        if wrote {
                            did_write = true;
                            last_write = timer.tick().0;
                        }
                    },
                    Err(()) => {
                        // TODO proper error handling, should we stop the loop?
                        println!("backend.write failed!");
                    },
                }

                // Remove events when they are done playing
                let mut i = 0;
                while i < events.len() {
                    if events[i].done {
                        events.swap_remove(i);
                    } else {
                        i += 1;
                    }
                }

                // Add new buffers/events
                for message in thread_receiver.try_recv() {
                    use self::MessageToAudioThread::*;
                    match message {
                        NewEvent { event } => {
                            events.push(event);
                        },
                        AddBuffer { buffer } => {
                            buffers.push(buffer);
                        },
                    }
                }

                let end = timer.tick().0;
                if did_write {
                    total_write_time += end - start;
                    write_count += 1;
                    average_write_time = Time(total_write_time.0 / write_count);
                }

                // Sleep for a bit, so this loop does not run constantly
                let write_interval = backend.write_interval();
                let before_sleep = timer.tick().0;
                let next_write = last_write + write_interval;
                let sleep_margin = Time::from_ms(2);

                if average_write_time > write_interval {
                    // TODO This means the computer we are running on is to slow to mix audio!
                    println!("Average write time is {} ns, but write interval is {} ns", average_write_time.0, write_interval.0);
                    return;
                }

                if next_write > before_sleep + sleep_margin {
                    let sleep_time = next_write - (before_sleep + sleep_margin);
                    thread::sleep(sleep_time.into());
                    let after_sleep = timer.tick().0;

                    if next_write + (write_interval - average_write_time) < after_sleep {
                        // TODO properly handle this case
                        println!(
                            "thread::sleep took to long! Should sleep to {} s, but slept until {} s",
                            next_write.as_secs_float(), after_sleep.as_secs_float(),
                        );
                    }
                }
            }
        });

        AudioSystem {
            broken: false,
            next_buffer_handle: 0,
            sender,
            receiver,
        }
    }

    pub fn tick(&mut self) {
        use self::MessageFromAudioThread::*;
        for message in self.receiver.try_recv() {
            match message {
                CriticalError => {
                    self.broken = true;
                },
            }
        }
    }

    pub fn play(&mut self, buffer: BufferHandle) {
        if self.broken {
            return;
        }

        let event = Event {
            start_frame: 0,
            done: false,
            buffer,
        };

        let message = MessageToAudioThread::NewEvent { event };
        let broken = self.sender.send(message).is_err();
        if broken {
            self.broken = true;
        }
    }

    pub fn add_buffer(&mut self, buffer: AudioBuffer) -> BufferHandle {
        if self.broken {
            return 0;
        }

        let message = MessageToAudioThread::AddBuffer { buffer };
        let broken = self.sender.send(message).is_err();
        if broken {
            self.broken = true;
        }

        let handle = self.next_buffer_handle;
        self.next_buffer_handle += 1;
        return handle;
    }
}

#[derive(Clone)]
pub struct AudioBuffer {
    pub channels: u32,
    pub sample_rate: u32,
    pub data: Vec<SampleData>,
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
    pub buffer: BufferHandle,
}

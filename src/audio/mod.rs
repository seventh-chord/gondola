
//! Experimental: custom audio stuff

// NB (Morten, 04.10.17)
// Regarding terminology
// A "sample" is a single i16 (Or whatever `SampleData` is): i16
// A "frame" is one i16 per channel:  (left, right): (i16, i16)

// NB (Morten, 8.10.17)
// We currently only output the first channel of a sound file in the mixer. If a stereo sound is
// submitted, we just ignore the second channel.

use std::ptr;
use std::thread;
use std::sync::mpsc;

use window::Window;
use time::{Time, Timer};

// Different platforms
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux::*;

pub mod wav;

const OUTPUT_CHANNELS: u32 = 2;
const OUTPUT_SAMPLE_RATE: u32 = 48000;
type SampleData = i16;
type Balance = [f32; OUTPUT_CHANNELS as usize];
type BufferHandle = usize;

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
    pub balance: Balance,
    pub speed: f32,
}



pub struct AudioSystem {
    next_buffer_handle: BufferHandle,

    pub state: AudioSystemState,
    has_printed_error: bool,

    receiver: mpsc::Receiver<AudioError>,
    sender: mpsc::Sender<MessageToAudioThread>,
}

pub enum AudioSystemState {
    Ok,
    AudioThreadDown,
    CriticalError(AudioError),
}

impl AudioSystemState {
    pub fn is_ok(&self) -> bool {
        match *self {
            AudioSystemState::Ok => true,
            _ => false,
        }
    }
}

enum MessageToAudioThread {
    NewEvent { event: Event },
    AddBuffer { buffer: AudioBuffer },
}

impl AudioSystem {
    pub fn initialize(window: &Window) -> AudioSystem {
        #[cfg(target_os = "windows")]
        let window_handle = window.window_handle() as usize; // Stupid hack
        #[cfg(not(target_os = "windows"))]
        let _ = window; // To ignore the warning

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
                Err(error) => {
                    let _ = thread_sender.send(error);
                    return;
                },
            };

            let mut frame_counter = 0;
            let mut timer = Timer::new();

            let mut buffers = Vec::with_capacity(100);
            let mut events  = Vec::with_capacity(100);
            let mut mix_scratch_buffer = Vec::new();

            let mut last_write = Time::ZERO;
            let mut average_write_time = Time::ZERO;
            let mut total_write_time = Time::ZERO;
            let mut write_count = 0;

            loop {
                let mut did_write = false;

                let start = timer.tick().0;

                // Actually update audio output
                let write_result = backend.write(
                    &mut frame_counter,
                    |frame, samples| {
                        self::mix(
                            &buffers, &mut events,
                            &mut mix_scratch_buffer,
                            frame, samples
                        );
                    },
                );

                match write_result {
                    Ok(wrote) => {
                        if wrote {
                            did_write = true;
                            last_write = start;
                        }
                    },

                    Err(error) => {
                        // The audio backends should handle non-critical errors themselves. If we
                        // get an error here, we just report it and abort.
                        let _ = thread_sender.send(error);
                        return;
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
                for message in thread_receiver.try_iter() {
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
                        // Eh: this triggered a couple of times without any audio discontinuities,
                        // so somethign is afoot
                        println!(
                            "thread::sleep took to long! Should sleep to {} s, but slept until {} s",
                            next_write.to_secs_f32(), after_sleep.to_secs_f32(),
                        );
                    }
                }
            }
        });

        AudioSystem {
            next_buffer_handle: 0,
            state: AudioSystemState::Ok,
            has_printed_error: false,
            sender,
            receiver,
        }
    }

    pub fn tick(&mut self) {
        if !self.state.is_ok() {
            return;
        }

        if let Ok(error) = self.receiver.try_recv() {
            self.state = AudioSystemState::CriticalError(error);
        }
    }

    pub fn play(&mut self, buffer: BufferHandle, balance: Balance, speed: f32) {
        if !self.state.is_ok() {
            return;
        }

        let event = Event {
            start_frame: 0,
            done: false,
            buffer,
            balance,
            speed,
        };

        let message = MessageToAudioThread::NewEvent { event };
        let send_result = self.sender.send(message);
        if send_result.is_err() {
            self.state = AudioSystemState::AudioThreadDown;
        }
    }

    pub fn add_buffer(&mut self, buffer: AudioBuffer) -> BufferHandle {
        if !self.state.is_ok() {
            return 0;
        }

        let message = MessageToAudioThread::AddBuffer { buffer };
        let send_result = self.sender.send(message);
        if send_result.is_err() {
            self.state = AudioSystemState::AudioThreadDown;
        }

        let handle = self.next_buffer_handle;
        self.next_buffer_handle += 1;
        return handle;
    }

    /// If `state` is not `Ok` this prints a detailed error message for the current `state`. If
    /// this function is called multiple times, it will only print once.
    pub fn print_potential_error(&mut self) {
        use self::AudioSystemState::*;
        use self::AudioError::*;

        if self.has_printed_error {
            return;
        }

        match self.state {
            AudioThreadDown => {
                println!("Audio thread stopped unexpectedly")
            },

            CriticalError(Other { ref message }) => {
                println!("Critical error in audio system: {}", message);
            },

            CriticalError(BadReturn { ref function_name, error_code, line, file }) => {
                println!(
                    "Critical error in audio system at {}:{}: `{}` returned {} unexpectedly",
                    file, line,
                    function_name,
                    error_code,
                );
            },

            Ok => return,
        }

        self.has_printed_error = true;
    }
}

// This is called through a callback from ´backend::write´
fn mix(
    buffers: &[AudioBuffer], 
    events: &mut [Event],
    scratch_buffer: &mut Vec<f32>,

    target_start_frame: u64,
    samples: &mut [SampleData],
) {
    assert!(samples.len() % (OUTPUT_CHANNELS as usize) == 0);
    let frame_count = (samples.len() / (OUTPUT_CHANNELS as usize)) as u64;
    let target_end_frame = target_start_frame + frame_count;

    scratch_buffer.clear();
    scratch_buffer.reserve(samples.len());
    unsafe {
        scratch_buffer.set_len(samples.len());
        ptr::write_bytes(scratch_buffer.as_mut_ptr(), 0, samples.len());
    }

    for event in events.iter_mut() {
        let ref buffer = buffers[event.buffer];

        if event.start_frame == 0 {
            // Start the sound playing now
            event.start_frame = target_start_frame;
        }


        let buffer_rate = (buffer.sample_rate as f32 / event.speed) as u32;
        let output_rate = OUTPUT_SAMPLE_RATE;
        
        #[inline(always)]
        fn convert_frames(frames: u64, from_rate: u32, to_rate: u32) -> u64 {
            (frames * (to_rate as u64)) / (from_rate as u64)
        }

        // How many frames the buffer would have if it was at the output sample rate
        let output_buffer_frames = convert_frames(buffer.frames(), buffer_rate, output_rate);

        let event_start_frame = event.start_frame;
        let event_end_frame = event_start_frame + output_buffer_frames;

        if event_end_frame < target_start_frame {
            event.done = true;
        }

        let start_frame = Ord::max(event_start_frame, target_start_frame);
        let end_frame   = Ord::min(event_end_frame, target_end_frame);

        if start_frame >= end_frame {
            // No part of this event fit into the frame window of the given samples
            continue;
        }

        // Actually mix the event into the scratch buffer
        let read_data = {
            let buffer_frame_range = (
                convert_frames(start_frame - event_start_frame, output_rate, buffer_rate),
                convert_frames(end_frame - event_start_frame,   output_rate, buffer_rate),
            );
            let a = buffer_frame_range.0 as usize * buffer.channels as usize;
            let b = buffer_frame_range.1 as usize * buffer.channels as usize;
            let b = Ord::min(b, buffer.data.len() - 1); // Sometimes happens due to rounding or smth
            &buffer.data[a..b]
        };

        let write_data = {
            let a = (start_frame - target_start_frame) as usize * OUTPUT_CHANNELS as usize;
            let b = (end_frame - target_start_frame) as usize   * OUTPUT_CHANNELS as usize;
            &mut scratch_buffer[a..b]
        };

        for frame in 0..(end_frame - start_frame) {
            for output_channel in 0..(OUTPUT_CHANNELS as usize) {
                let read_frame = convert_frames(frame, output_rate, buffer_rate);

                // Compute the fractional part of ´read_frame´
                let t = (10000*frame * (buffer_rate as u64)) / (output_rate as u64);
                let t = (t - read_frame*10000) as f32 / 10000.0;

                let prev_read_pos = (read_frame as usize)*(buffer.channels as usize);
                let last = read_data.len() - 1;
                let prev_read_pos = Ord::min(prev_read_pos, last); // Sometimes happens due to rounding
                let next_read_pos = Ord::min(prev_read_pos + buffer.channels as usize, last);

                // Linearly interpolate to find the proper sample value. In theory, this gives us a
                // better result, but in practice it doesn't matter: I can't hear the difference.
                let prev_sample = read_data[prev_read_pos] as f32;
                let next_sample = read_data[next_read_pos] as f32;
                let sample = prev_sample*(1.0 - t) + next_sample*t;

                let volume = event.balance[output_channel];

                let write_pos = (frame as usize)*(OUTPUT_CHANNELS as usize) + output_channel;
                write_data[write_pos] += sample*volume;
            }
        }
    }

    // Write the scratchbuffer back into the provided sample buffer
    let min = SampleData::min_value() as f32;
    let max = SampleData::max_value() as f32;

    for (index, &sample) in scratch_buffer.iter().enumerate() {
        let clipped = clamp(sample, (min, max));
        samples[index] = clipped as i16;
    }
}

#[inline(always)]
fn clamp<T: PartialOrd + Copy>(v: T, range: (T, T)) -> T {
    if v > range.1 {
        range.1
    } else if v < range.0 {
        range.0
    } else {
        v
    }
}


/// Most of these errors are critical, we are not expecting to recover from them. If they happen, we
/// just give up on sound completly. Because of that, we favour human-readable error formats (strings).
pub enum AudioError {
    Other { message: String }, 
    
    // Some function returned a bad value
    BadReturn { 
        function_name: String,
        error_code: i64,
        line: u32,
        file: &'static str,
    },
}

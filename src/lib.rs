
//! A semi-safe, semi-stateless wrapper around OpenGL 3.3 Core. This library provides various
//! utilities to make using OpenGL 3.3 safer. It uses rust's type system to encode some information
//! which helps prevent common errors. This library is primarily intended to be used for games,
//! but you can also use it to create other graphics applications.
//!
//! Some points to get started:
//!
//!  - Use [`gondola::run`] to launch your game.
//!  - Use a [`VertexBuffer`] to do basic drawing.
//!  - Use [`GameState::gen_input_manager`] to get access to keyboard/mouse state.
//!
//! [`GameState::gen_input_manager`]: struct.GameState.html#method.gen_input_manager
//! [`VertexBuffer`]:                 buffer/struct.VertexBuffer.html
//! [`gondola::run`]:                 fn.run.html

extern crate gl;
extern crate glutin;
extern crate winit;
extern crate libc;

extern crate rusttype;
extern crate png;
#[macro_use]
extern crate bitflags;
#[cfg(feature = "serialize")]
extern crate serde;

extern crate cable_math;

mod color;
mod input;

pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod graphics;
pub mod framebuffer;
pub mod font;
pub mod draw_group;
//pub mod ui; // Temporarily disabled. Broken due to changes in font code. Should be rewritten to use draw_group

use std::time::{Instant, Duration};
use std::sync::mpsc;
use std::ops::{Add, Sub, AddAssign, SubAssign};
use std::thread;

use cable_math::Vec2;

pub use color::*;
pub use input::*;
pub use draw_group::DrawGroup;

/// The most generic result type possible. Used in top-level
pub type GameResult<T> = Result<T, Box<std::error::Error>>;

/// Creates a new window and runs the given game in this window. This function does 
/// not return until the game exits.
///
/// # Example
/// ```rust,no_run
/// extern crate gondola;
///
/// use gondola::{Game, GameResult, GameState, Timing};
///
/// fn main() {
///     gondola::run::<Pong>();
/// }
///
/// struct Pong {
///     // All data neded for game is defined here
/// }
///
/// impl Game for Pong {
///     fn setup(state: &mut GameState) -> GameResult<Pong> {
///         Ok(Pong {})
///     }
///
///     fn update(&mut self, delta: Timing, state: &mut GameState) {
///         // All logic goes here.
///     }
///
///     fn draw(&mut self, state: &GameState) {
///         // All rendering goes here
///     }
/// }
/// ```
pub fn run<T: Game + Sized>() {
    let event_loop = glutin::EventsLoop::new();

    // We can use conditional compilation to set this for other platforms
    let gl_request = glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3));

    // Create window
    let result = glutin::WindowBuilder::new()
        .with_title(T::name())

        // Is enabled by default on some platforms, and often controlled by e.g.
        // environment variables
        // linux + nvidia:  __GL_SYNC_TO_VBLANK=1|0
        //      1 forces vsync on (default)
        //      0 disables vsync, but with_vsync still enables it 
//        .with_vsync() 

        .with_srgb(Some(false))

        .with_gl(gl_request)
        .with_gl_profile(glutin::GlProfile::Core)

        .build(&event_loop);

    let window = match result {
        Ok(window) => window,
        Err(err) => {
            println!("Failed to open window:\n{}", err);
            panic!();
        },
    };

    unsafe {
        window.make_current().unwrap();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }
    window.set_title(T::name());

    // Use a separate thread to read events, and send them back to the main thread
    let (main_event_sender, main_event_receiver) = mpsc::channel::<glutin::Event>();
    thread::spawn(move|| {
        event_loop.run_forever(|event| {
            let send_result = main_event_sender.send(event);

            // If we receive a error the receiving end is gone, meaning that the main thread 
            // has stopped
            if send_result.is_err() {
                event_loop.interrupt();
            }
        });
    });

    // Generate platform specific data
    let platform = {
        #[cfg(target_os = "windows")]
        {
            use winit::os::windows::WindowExt;
            let hwnd_ptr = window.as_winit_window().get_hwnd();

            Platform {
                hwnd_ptr: hwnd_ptr,
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Platform {}
        }
    };

    // Set up game state
    let mut state = GameState::new(platform, window);

    state.screen_region = {
        let size: Vec2<_> = state.window.get_inner_size_pixels().unwrap().into();
        let size = size.as_f32();

        Region { min: Vec2::zero(), max: size }
    };
    graphics::viewport(state.screen_region);

    // Set up game
    let mut game = match T::setup(&mut state) {
        Err(err) => {
            println!("Failed to launch game:\n{}", err);
            panic!();
        },
        Ok(game) => game,
    };

    // We run a resize event here, because some platforms don't send a Resized event on startup.
    game.on_resize(&state);

    // Having a decent guess for the first frame resolves some stuttering issues
    let mut delta = Timing::from_ms(16); 

    'main_loop:
    loop {
        let start_time = Instant::now();

        // Events
        for event in main_event_receiver.try_iter() {
            let glutin::Event::WindowEvent { event, .. } = event; // Legacy api, woo

            match event {
                glutin::WindowEvent::Closed => break 'main_loop,
                glutin::WindowEvent::Resized(..) => {
                    let size: Vec2<_> = state.window.get_inner_size_pixels().unwrap().into();
                    let size = size.as_f32();

                    let changed = state.screen_region.size() != size;

                    if changed && size.x > 0.0 && size.y > 0.0 {
                        state.screen_region = Region {
                            min: Vec2::zero(),
                            max: size
                        };

                        graphics::viewport(state.screen_region);
                        game.on_resize(&state);
                    }
                },
                glutin::WindowEvent::Focused(focused) => {
                    state.focused = focused;

                    if focused {
                        let glutin_cursor_state = match state.cursor_state {
                            CursorState::Normal => glutin::CursorState::Normal,
                            CursorState::Hidden => glutin::CursorState::Hide,
                            CursorState::HiddenGrabbed => glutin::CursorState::Hide,
                            CursorState::Grabbed => glutin::CursorState::Grab,
                        }; 
                        state.window.set_cursor_state(glutin_cursor_state).unwrap();
                    } else {
                        state.window.set_cursor_state(glutin::CursorState::Normal).unwrap();
                    }
                },
                other => {
                    let custom_event = match other {
                        glutin::WindowEvent::MouseMoved(x, y) => {
                            let pos = Vec2::new(x as i32, y as i32);
                            let mut delta = pos - state.prev_cursor_pos;
                            state.prev_cursor_pos = pos;

                            // set_cursor_position creates mouse-moved events, we want to ignore those
                            if state.cursor_state == CursorState::HiddenGrabbed {
                                let center = state.screen_region.center().as_i32();
                                if pos == center {
                                    continue;
                                }

                                // The focused state is almost exclusively used to control the
                                // camera. When we don't have focus we don't want the camera to
                                // move even if the cursor is moving around above the screen.
                                // Because of this we just disable mouse deltas in that case.
                                if !state.focused {
                                    delta = Vec2::zero();
                                }
                            }

                            Event::MouseMoved { delta, pos }
                        },

                        // We usually dont want to create a custom event, so unless we have custom
                        // logic for a specific event type we just pass on the glutin event.
                        e => Event::GlutinEvent(e),
                    };
                    
                    // Send events to receivers, and remove those which are unable to receive.
                    state.event_sinks.retain(|sink| {
                        sink.send(custom_event.clone()).is_ok()
                    });
                },
            }
        }

        // Input state changes
        for input_state_change in state.state_change_request_receiver.try_iter() {
            match input_state_change {
                StateRequest::ChangeCursorState(new_state) => {
                    state.cursor_state = new_state;

                    let glutin_cursor_state = match new_state {
                        CursorState::Normal => glutin::CursorState::Normal,
                        CursorState::Hidden => glutin::CursorState::Hide,
                        CursorState::HiddenGrabbed => glutin::CursorState::Hide,
                        CursorState::Grabbed => glutin::CursorState::Grab,
                    };
                    state.window.set_cursor_state(glutin_cursor_state).unwrap();
                },
            }
        }

        if state.cursor_state == CursorState::HiddenGrabbed && state.focused {
            let center = state.screen_region.center().as_i32();
            state.window.set_cursor_position(center.x, center.y).unwrap();
        }

        // Logic and rendering
        game.update(delta, &mut state);
        game.draw(&state);
        state.window.swap_buffers().unwrap();
        graphics::print_errors();

        if state.exit {
            break 'main_loop;
        }

        delta = start_time.elapsed().into();

        // Calculate average framerate
        state.frame_accumulator += 1;
        state.delta_accumulator += delta;
        if state.delta_accumulator > Timing::from_ms(500) { // Update every half second
            let frames = state.frame_accumulator as f32;
            let seconds = state.delta_accumulator.as_secs_float();
            state.average_framerate = frames / seconds;

            state.frame_accumulator = 0;
            state.delta_accumulator = Timing::zero();
        }
    }

    game.close();
}

/// General info about the currently running game. Passed as a parameter to
/// most [`Game`](trait.Game.html) methods.
pub struct GameState {
    /// The region of the screen. `min` is (0, 0), `max` is (width, height).
    pub screen_region: Region,
    /// If set to true the game will exit after rendering.
    pub exit: bool,
    /// If true the game window currently has focus. 
    pub focused: bool,

    /// The number of frames that where displayed in the last second. This number is updated every 
    /// half second. Note that this is only an average; it does not reflect rapid fluctuations of 
    /// delta times.
    pub average_framerate: f32,
    // Used to calculate framerate
    frame_accumulator: u32,
    delta_accumulator: Timing,

    event_sinks: Vec<mpsc::Sender<Event>>,
    state_change_request_receiver: mpsc::Receiver<StateRequest>,
    state_change_request_sender: mpsc::Sender<StateRequest>,

    // To work around some input issue we need to track some input state here. Most input state
    // is however tracked by the input manager.
    prev_cursor_pos: Vec2<i32>,
    cursor_state: CursorState,

    platform: Platform,
    window: glutin::Window,
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
pub struct Platform {
    pub hwnd_ptr: *mut libc::c_void,
}

#[cfg(not(target_os = "windows"))]
#[derive(Clone)]
pub struct Platform {}

/// Used with [`gondola::run`](fn.run.html)
pub trait Game: Sized {
    /// Called before the main loop. Resources and initial state should be set up here.
    fn setup(state: &mut GameState) -> GameResult<Self>;
    /// Called once every frame, before drawing.
    fn update(&mut self, delta: Timing, state: &mut GameState);
    /// Called once every frame, after updating.
    fn draw(&mut self, state: &GameState);

    /// Called whenever the game window is resized. This function is guaranteed to be called
    /// once whenever the game is started. When called, this function is allways called before
    /// `update` and `resize`.
    fn on_resize(&mut self, _state: &GameState) {}
    /// Called after the main game loop exists. This method is not called if the main
    /// loop `panic!`s. Most simple games don't need any logic here.
    fn close(&mut self) {} 

    /// The name dipslayed in the title bar of the games window.
    fn name() -> &'static str { "Unnamed game (Override Game::name to change title)" }
}

impl GameState {
    fn new(platform: Platform, window: glutin::Window) -> GameState {
        let (state_change_request_sender, state_change_request_receiver) = mpsc::channel();

        GameState {
            screen_region: Region::default(),
            exit: false,

            average_framerate: -1.0,
            frame_accumulator: 0,
            delta_accumulator: Timing::zero(),

            focused: false, // We get a Focused(true) event once the window opens

            event_sinks: Vec::new(),
            state_change_request_receiver: state_change_request_receiver,
            state_change_request_sender: state_change_request_sender,

            prev_cursor_pos: Vec2::zero(),
            cursor_state: CursorState::Normal,

            platform,
            window,
        }
    }

    /// Generates a new receiver for glutin events. All events not consumed by the
    /// game itself will be received by this receiver. Note that all receiver receive
    /// all events, there is no notion of event consumption.
    pub fn gen_event_receiver(&mut self) -> mpsc::Receiver<Event> {
        let (sender, receiver) = mpsc::channel();
        self.event_sinks.push(sender);
        receiver
    }

    /// Generates a new sender to transmit state change requests to the game core asynchronously.
    pub fn gen_request_sender(&self) -> mpsc::Sender<StateRequest> {
        self.state_change_request_sender.clone()
    }

    /// Creates a new input manager which can be used to access the state of input devices.
    pub fn gen_input_manager(&mut self) -> InputManager {
        let receiver = self.gen_event_receiver();
        let sender = self.gen_request_sender();
        InputManager::new(receiver, sender)
    }

    /// Returns a struct, the contents of which vary based on platform. On windows, this struct
    /// for example contains the native window handle (HWND).
    pub fn platform_specific_data(&self) -> Platform {
        self.platform.clone()
    }

    /// Chagnes the title of this window
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }
}

/// Because glutin does not handle some things in a very elegant fashion we have to work around
/// some of its events with custom events.
#[derive(Debug, Clone)]
pub enum Event {
    GlutinEvent(glutin::WindowEvent),
    MouseMoved {
        delta: Vec2<i32>,
        pos: Vec2<i32>,
    },
}

#[derive(Debug, Clone)]
pub enum StateRequest {
    ChangeCursorState(CursorState),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timing(u64); 

impl Timing {
    pub fn zero() -> Timing { Timing(0) }

    pub fn from_ms(ms: u64) -> Timing { Timing(ms * 1_000_000) } 
    pub fn from_secs(s: u64) -> Timing { Timing(s * 1_000_000_000) } 
    pub fn from_secs_float(s: f32) -> Timing { Timing((s * 1_000_000_000.0) as u64) } 

    /// Converts this timing to seconds, truncating any overflow. 1.999 ms will be converted to 1 ms.
    pub fn as_ms(self) -> u64 { self.0 / 1_000_000 }

    /// Converts this timing to seconds, truncating any overflow. 1.999 seconds will be converted to 1.
    pub fn as_secs(self) -> u64 { self.0 / 1_000_000_000 }

    pub fn as_secs_float(self) -> f32 { self.0 as f32 / 1_000_000_000.0 }

    pub fn max(self, other: Timing) -> Timing {
        ::std::cmp::max(self, other)
    }

    pub fn min(self, other: Timing) -> Timing {
        ::std::cmp::min(self, other)
    }
}

impl Add for Timing {
    type Output = Timing;
    fn add(self, rhs: Timing) -> Timing {
        Timing(self.0 + rhs.0)
    }
}

impl Sub for Timing {
    type Output = Timing;
    fn sub(self, rhs: Timing) -> Timing {
        Timing(self.0 - rhs.0)
    }
}

impl AddAssign for Timing {
    fn add_assign(&mut self, rhs: Timing) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Timing {
    fn sub_assign(&mut self, rhs: Timing) {
        self.0 -= rhs.0;
    }
}

impl From<Duration> for Timing {
    fn from(d: Duration) -> Timing {
        Timing(d.as_secs()*1_000_000_000 + (d.subsec_nanos() as u64))
    }
}

impl From<Timing> for Duration {
    fn from(t: Timing) -> Duration {
        let nanos = t.0 % 1_000_000_000;
        let secs = t.0 / 1_000_000_000;
        Duration::new(secs, nanos as u32)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Region {
    pub min: Vec2<f32>,
    pub max: Vec2<f32>,
}

impl Region {
    pub fn center(&self) -> Vec2<f32> { (self.min + self.max) / 2.0 } 

    pub fn width(&self) -> f32        { self.max.x - self.min.x }
    pub fn height(&self) -> f32       { self.max.y - self.min.y }

    pub fn size(&self) -> Vec2<f32>   { self.max - self.min }

    /// Checks if the given point is inside this region.
    pub fn contains(&self, p: Vec2<f32>) -> bool {
        p.x > self.min.x && p.x < self.max.x &&
        p.y > self.min.y && p.y < self.max.y
    }

    /// Width divided by height.
    pub fn aspect(&self) -> f32 {
        let size = self.size();
        size.x / size.y
    }

    /// Swaps `min` and `max` along the y axis
    pub fn flip_y(self) -> Region {
        Region {
            min: Vec2::new(self.min.x, self.max.y),
            max: Vec2::new(self.max.x, self.min.y),
        }
    }

    /// Swaps `min` and `max` along the x axis
    pub fn flip_x(self) -> Region {
        Region {
            min: Vec2::new(self.max.x, self.min.y),
            max: Vec2::new(self.min.x, self.max.y),
        }
    }

    /// Returns the region in which this region overlaps the given other region. This might produce
    /// a negative region.
    pub fn overlap(self, other: Region) -> Region {
        Region {
            min: Vec2 {
                x: f32::max(self.min.x, other.min.x),
                y: f32::max(self.min.y, other.min.y),
            },
            max: Vec2 {
                x: f32::min(self.max.x, other.max.x),
                y: f32::min(self.max.y, other.max.y),
            },
        }
    }
}

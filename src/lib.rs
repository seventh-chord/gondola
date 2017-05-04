
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
extern crate png;
extern crate cable_math;
extern crate rusttype;
#[macro_use]
extern crate bitflags;
#[cfg(feature = "serialize")]
extern crate serde;

mod color;
mod input;

pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod util;
pub mod framebuffer;
pub mod font;
pub mod ui;

pub use color::*;
pub use input::*;
pub use util::graphics;

use cable_math::Vec2;
use std::time::{Instant, Duration};
use std::sync::mpsc;
use std::thread;

/// The most generic result type possible. Used in top-level
pub type GameResult<T> = Result<T, Box<std::error::Error>>;

/// Creates a new window and runs the given game in this window. This function does 
/// not return until the game exits.
///
/// # Example
/// ```rust,no_run
/// extern crate gondola;
///
/// use gondola::{Game, GameResult, GameState};
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
///     fn update(&mut self, delta: u32, state: &mut GameState) {
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

    // Create window
    let result = glutin::WindowBuilder::new()
        .with_title(T::name())
//        .with_vsync()
        .with_srgb(Some(false))
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

    // Set up game state
    let mut state = GameState::new();
    state.win_size = {
        let (width, height) = window.get_inner_size_pixels().unwrap();
        Vec2::new(width, height)
    };
    graphics::viewport(0, 0, state.win_size.x, state.win_size.y);

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

    let mut delta: u64 = 16; 

    'main_loop:
    loop {
        let start_time = Instant::now();

        // Events
        for event in main_event_receiver.try_iter() {
            let glutin::Event::WindowEvent { event, .. } = event; // Legacy api, woo

            match event {
                glutin::WindowEvent::Closed => break 'main_loop,
                glutin::WindowEvent::Resized(..) => {
                    let (width, height) = window.get_inner_size_pixels().unwrap();
                    let changed = state.win_size.x != width || state.win_size.y != height;

                    if width != 0 && height != 0 && changed {
                        state.win_size = Vec2::new(width, height);
                        graphics::viewport(0, 0, state.win_size.x, state.win_size.y);
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
                        window.set_cursor_state(glutin_cursor_state).unwrap();
                    } else {
                        window.set_cursor_state(glutin::CursorState::Normal).unwrap();
                    }
                },
                other => {
                    let custom_event = match other {
                        glutin::WindowEvent::MouseMoved(x, y) => {
                            let pos = Vec2::new(x as i32, y as i32);
                            let mut delta = pos - state.prev_cursor_pos;
                            state.prev_cursor_pos = pos;

                            // set_cursor_position creates mose-moved events, we want to ignore those
                            if state.cursor_state == CursorState::HiddenGrabbed {
                                let center = state.win_size / 2;
                                let center = Vec2::new(center.x as i32, center.y as i32);
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
                    window.set_cursor_state(glutin_cursor_state).unwrap();
                },
            }
        }

        if state.cursor_state == CursorState::HiddenGrabbed && state.focused {
            let center = state.win_size / 2;
            let center = Vec2::new(center.x as i32, center.y as i32);
            window.set_cursor_position(center.x, center.y).unwrap();
        }

        // Logic and rendering
        game.update(delta as u32, &mut state);
        game.draw(&state);
        window.swap_buffers().unwrap();
        graphics::print_errors();

        if state.exit {
            break 'main_loop;
        }

        // Timing
        if let Some(target_delta) = state.target_delta {
            let target_delta = Duration::from_millis(target_delta as u64);
            let elapsed = start_time.elapsed();
            if elapsed < target_delta {
                std::thread::sleep(target_delta - elapsed); // This is not very precice :/
            }
        }
        let delta_dur = start_time.elapsed();
        delta = delta_dur.as_secs()*1000 + (delta_dur.subsec_nanos() as u64)/1000000;

        // Calculate average framerate
        state.frame_accumulator += 1;
        state.delta_accumulator += delta as u32;
        if state.delta_accumulator > 500 { // Update every half second
            let frames = state.frame_accumulator as f32;
            let time = (state.delta_accumulator as f32) * 0.001;

            state.average_framerate = frames / time;

            state.frame_accumulator = 0;
            state.delta_accumulator = 0;
        }
    }

    game.close();
}

/// General info about the currently running game. Passed as a parameter to
/// most [`Game`](trait.Game.html) methods.
pub struct GameState {
    /// The size of the window in which this game is running, in pixels.
    pub win_size: Vec2<u32>,
    /// If set to true the game will exit after rendering.
    pub exit: bool,
    /// If true the game window currently has focus. 
    pub focused: bool,

    /// The number of milliseconds per frame this game should aim to run at. Set to 16
    /// for 60 fps. If the main loop takes less time than this amount the game will
    /// sleep until a total of `target_delta` has ellapsed. If set to `None` the game will
    /// never sleep, and run as fast as possible.
    pub target_delta: Option<u32>,
    /// The number of frames that where displayed in the last second. This number is updated every 
    /// half second. Note that this is only an average; it does not reflect rapid fluctuations of 
    /// delta times.
    pub average_framerate: f32,
    // Used to calculate framerate
    frame_accumulator: u32,
    delta_accumulator: u32,

    event_sinks: Vec<mpsc::Sender<Event>>,
    state_change_request_receiver: mpsc::Receiver<StateRequest>,
    state_change_request_sender: mpsc::Sender<StateRequest>,

    // To work around some input issue we need to track some input state here. Most input state
    // is however tracked by the input manager.
    prev_cursor_pos: Vec2<i32>,
    cursor_state: CursorState,
}

/// Used with [`gondola::run`](fn.run.html)
pub trait Game: Sized {
    /// Called before the main loop. Resources and initial state should be set up here.
    fn setup(state: &mut GameState) -> GameResult<Self>;
    /// Called once every frame, before drawing.
    fn update(&mut self, delta: u32, state: &mut GameState);
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
    fn new() -> GameState {
        let (state_change_request_sender, state_change_request_receiver) = mpsc::channel();

        GameState {
            win_size: Vec2::zero(),
            exit: false,
            target_delta: Some(15),

            average_framerate: -1.0,
            frame_accumulator: 0,
            delta_accumulator: 0,

            focused: false, // We get a Focused(true) event once the window opens

            event_sinks: Vec::new(),
            state_change_request_receiver: state_change_request_receiver,
            state_change_request_sender: state_change_request_sender,

            prev_cursor_pos: Vec2::zero(),
            cursor_state: CursorState::Normal,
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


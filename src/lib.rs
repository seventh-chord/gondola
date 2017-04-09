
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
mod matrix_stack;
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
pub use matrix_stack::*;
pub use util::graphics;

use cable_math::Vec2;
use std::time::{Instant, Duration};
use std::sync::mpsc;

/// The most generic result type possible. Used in top-level
pub type GameResult<T> = Result<T, Box<std::error::Error>>;

/// Creates a new window and runs the given game in this window. This function does 
/// not return until the game exits.
///
/// # Example
/// ```rust,no_run
/// extern crate gondola;
///
/// use gondola::{Game, GameResult, GameState, MatrixStack};
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
///     fn draw(&mut self, state: &GameState, mut mat_stack: &mut MatrixStack) {
///         // All rendering goes here
///     }
/// }
/// ```
pub fn run<T: Game + Sized>() {
    // Create window
    let window = glutin::Window::new().unwrap();
    unsafe {
        window.make_current().unwrap();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }
    window.set_title(T::name());

    // Set up game state
    let mut state = GameState::new();
    state.win_size = {
        let (width, height) = window.get_inner_size_pixels().unwrap();
        Vec2::new(width, height)
    };
    graphics::viewport(0, 0, state.win_size.x, state.win_size.y);

    let mut mat_stack = MatrixStack::new();

    // Set up game
    let mut game = match T::setup(&mut state) {
        Err(err) => {
            println!("Failed to launch game:\n{}", err);
            panic!();
        },
        Ok(game) => game,
    };

    let mut delta: u64 = 16;

    'main_loop:
    loop {
        let start_time = Instant::now();

        // Events
        for event in window.poll_events() {
            match event {
                glutin::Event::Closed => break 'main_loop,
                glutin::Event::Resized(..) => {
                    let (width, height) = window.get_inner_size_pixels().unwrap();
                    if width != 0 && height != 0 {
                        state.win_size = Vec2::new(width, height);
                        graphics::viewport(0, 0, state.win_size.x, state.win_size.y);
                        game.on_resize(&state);
                    }
                }
                other => {
                    // Send events to receivers, and remove those which are unable to receive.
                    state.event_sinks.retain(|sink| {
                        sink.send(other.clone()).is_ok()
                    });
                },
            }
        }

        // Logic and rendering
        game.update(delta as u32, &mut state);
        game.draw(&state, &mut mat_stack);
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

    event_sinks: Vec<mpsc::Sender<glutin::Event>>,
}

/// Used with [`gondola::run`](fn.run.html)
pub trait Game: Sized {
    /// Called before the main loop. Resources and initial state should be set up here.
    fn setup(state: &mut GameState) -> GameResult<Self>;
    /// Called once every frame, before drawing.
    fn update(&mut self, delta: u32, state: &mut GameState);
    /// Called once every frame, after updating.
    fn draw(&mut self, state: &GameState, mat_stack: &mut MatrixStack);

    /// Called whenever the game window is resized
    fn on_resize(&mut self, _state: &GameState) {}
    /// Called after the main game loop exists. This method is not called if the main
    /// loop `panic!`s.
    fn close(&mut self) {} // Most simple games dont need any special logic here

    fn name() -> &'static str { "Unnamed game (Override Game::name to change title)" }
}

impl GameState {
    fn new() -> GameState {
        GameState {
            win_size: Vec2::zero(),
            exit: false,
            target_delta: Some(15),

            average_framerate: -1.0,
            frame_accumulator: 0,
            delta_accumulator: 0,

            event_sinks: Vec::new(),
        }
    }

    /// Generates a new receiver for glutin events. All events not consumed by the
    /// game itself will be received by this receiver. Note that all receiver receive
    /// all events, there is no notion of event consumption.
    pub fn gen_event_receiver(&mut self) -> mpsc::Receiver<glutin::Event> {
        let (sender, receiver) = mpsc::channel();
        self.event_sinks.push(sender);
        receiver
    }

    /// Creates a new input manager which can be used to access the state of input devices.
    pub fn gen_input_manager(&mut self) -> InputManager {
        let receiver = self.gen_event_receiver();
        InputManager::new(receiver)
    }
}



extern crate gl;
extern crate glutin;
extern crate png;
extern crate cable_math;
extern crate rusttype;

pub mod color;
pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod matrix_stack;
pub mod util;
pub mod framebuffer;
pub mod font;
pub mod input;
pub mod ui;

pub use color::*;
pub use input::*;
pub use matrix_stack::*;
pub use util::graphics;

use cable_math::Vec2;
use glutin::*;
use std::io;
use std::time::{Instant, Duration};

/// Creates a new window and runs the given game in this window. This function does 
/// not return until the game exits.
///
/// # Example
/// ```rust
/// fn main() {
///     gondola::run::<Pong>();
/// }
///
/// struct Pong {
///     // All data neded for game is defined here
/// }
///
/// impl Game for Pong {
///     fn setup(state: &mut GameState) -> io::Result<Pong> {
///         Ok(Pong {})
///     }
///
///     fn update(&mut self, delta: u32, state: &mut GameState, input: &InputManager) {
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
    state.window_size = {
        let (width, height) = window.get_inner_size_pixels().unwrap();
        Vec2::new(width, height)
    };
    graphics::viewport(0, 0, state.window_size.x, state.window_size.y);

    let mut input = InputManager::new();

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
        input.update();
        for event in window.poll_events() {
            match event {
                Event::Closed => break 'main_loop,
                Event::Resized(..) => {
                    let (width, height) = window.get_inner_size_pixels().unwrap();
                    if width != 0 && height != 0 {
                        state.window_size = Vec2::new(width, height);
                        graphics::viewport(0, 0, state.window_size.x, state.window_size.y);
                        game.on_resize(&state);
                    }
                }
                other => input.handle_event(other),
            }
        }

        // Logic and rendering
        game.update(delta as u32, &mut state, &input);
        game.draw(&state, &mut mat_stack);
        window.swap_buffers().unwrap();
        graphics::print_errors();

        if state.exit {
            break 'main_loop;
        }

        // Timing
        let elapsed = start_time.elapsed();
        let target_delta = Duration::from_millis(state.target_delta as u64);
        if state.target_delta > 0 && elapsed < target_delta {
            std::thread::sleep(target_delta - elapsed); // This is not very precice :/
        }
        let delta_dur = start_time.elapsed();
        delta = delta_dur.as_secs()*1000 + (delta_dur.subsec_nanos() as u64)/1000000;
    }

    game.close();
}

/// General info about the currently running game. Passed as a parameter to
/// most [`Game`](trait.Game.html) methods.
pub struct GameState {
    /// The size of the window in which this game is running, in pixels.
    pub window_size: Vec2<u32>,
    /// If set to true the game will exit after rendering.
    pub exit: bool,
    /// The number of milliseconds per frame this game should aim to run at. Set to 16
    /// for 60 fps. If the main loop takes less time than this amount the game will
    /// sleep until a total of `target_delta` has ellapsed. If set to `0` the game will
    /// never sleep.
    pub target_delta: u32,
}

/// Used with [`gondola::run`](fn.run.html)
pub trait Game: Sized {
    /// Called before the main loop. Resources and initial state should be set up here.
    fn setup(state: &mut GameState) -> io::Result<Self>;
    /// Called once every frame, before drawing.
    fn update(&mut self, delta: u32, state: &mut GameState, input: &InputManager);
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
    pub fn new() -> GameState {
        GameState {
            window_size: Vec2::zero(),
            exit: false,
            target_delta: 15,
        }
    }
}


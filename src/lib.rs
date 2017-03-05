
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

pub use color::*;
pub use util::graphics;

use cable_math::Vec2;
use glutin::*;
use std::io;
use std::time::{Instant, Duration};

pub fn run<T: Game + Sized>() {
    // Create window
    let window = glutin::Window::new().unwrap();
    unsafe {
        window.make_current().unwrap();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }

    // Set up game state
    let mut state = GameState::new();
    state.window_size = {
        let window_size = window.get_inner_size_pixels().unwrap();
        Vec2::new(window_size.0, window_size.1)
    };
    graphics::viewport(0, 0, state.window_size.x, state.window_size.y);

    // Set up game
    let mut game = match T::setup(&mut state) {
        Err(err) => {
            println!("Failed to launch game:\n{}", err);
            panic!();
        },
        Ok(game) => game,
    };

    let mut delta: u64 = 16;
    let target_delta = Duration::from_millis(14);

    'main_loop:
    loop {
        let start_time = Instant::now();

        // Events
        state.input.update();
        for event in window.poll_events() {
            match event {
                Event::Closed => break 'main_loop,
                Event::Resized(width, height) => {
                    if width != 0 && height != 0 {
                        state.window_size = Vec2::new(width, height);
                        graphics::viewport(0, 0, state.window_size.x, state.window_size.y);
                        game.on_resize(&state);
                    }
                }
                other => state.input.handle_event(other),
            }
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
        let elapsed = start_time.elapsed();
        if elapsed < target_delta {
            std::thread::sleep(target_delta - elapsed); // This is not very precice :/
        }
        let delta_dur = start_time.elapsed();
        delta = delta_dur.as_secs()*1000 + (delta_dur.subsec_nanos() as u64)/1000000;
    }

    game.close();
}

pub struct GameState {
    window_size: Vec2<u32>,
    input: InputManager,
    exit: bool,
}

pub struct InputManager {
    mouse_pos: Vec2<f32>,
    mouse_delta: Vec2<f32>,
}

pub trait Game: Sized {
    fn setup(state: &mut GameState) -> io::Result<Self>;
    fn update(&mut self, delta: u32, state: &mut GameState);
    fn draw(&mut self, state: &GameState);

    fn on_resize(&mut self, state: &GameState) {}
    fn close(&mut self) {} // Most simple games dont need any special logic here
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            window_size: Vec2::zero(),
            input: InputManager::new(),
            exit: false
        }
    }

    /// Causes the game to exit. The game is exited after calls to `update` and 
    /// `draw` have returned.
    pub fn exit(&mut self) { self.exit = true; }

    pub fn input(&self) -> &InputManager { &self.input }
    pub fn window_size(&self) -> Vec2<u32> { self.window_size }
}

impl InputManager {
    fn new() -> InputManager {
        InputManager {
            mouse_pos: Vec2::zero(),
            mouse_delta: Vec2::zero(),
        }
    }

    fn update(&mut self) {
        self.mouse_delta = Vec2::zero();
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::MouseMoved(x, y) => {
                let old_mouse_pos = self.mouse_pos;
                self.mouse_pos = Vec2::new(x as f32, y as f32);
                self.mouse_delta = self.mouse_pos - old_mouse_pos;
            },
            _ => {},
        }
    }
    
    // Getters
    pub fn mouse_pos(&self)   -> Vec2<f32> { self.mouse_pos }
    pub fn mouse_delta(&self) -> Vec2<f32> { self.mouse_delta }
}


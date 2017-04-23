
//! Provides utilities for tracking the state of various input devices

use cable_math::Vec2;
use std::sync::mpsc;
use glutin::{Event, MouseButton, ElementState, MouseScrollDelta};

const MOUSE_KEYS: usize = 5;
const KEYBOARD_KEYS: usize = 256; // This MUST be `u8::max_value() + 1`

/// Manages keyboard and mouse input. This manager receives events from a
/// `mpsc::Receiver`. This means it can be created and used in other threads.
/// Note that [`InputManager::refresh`] should be called once per frame.
///
/// `InputManager`s are created by [`GameState::gen_input_manager`]
///
/// [`GameState::gen_input_manager`]: ../struct.GameState.html#method.gen_input_manager
/// [`InputManager::refresh`]:        struct.InputManager.html#method.refresh
pub struct InputManager {
    mouse_pos: Vec2<f32>,
    mouse_delta: Vec2<f32>,
    mouse_scroll: Vec2<f32>,
    mouse_states: [State; MOUSE_KEYS],
    keyboard_states: [State; KEYBOARD_KEYS],
    type_buffer: String,

    prev_event_count: usize, 
    event_source: mpsc::Receiver<Event>,
}

impl InputManager {
    pub fn new(event_source: mpsc::Receiver<Event>) -> InputManager {
        InputManager {
            mouse_pos: Vec2::zero(),
            mouse_delta: Vec2::zero(),
            mouse_scroll: Vec2::zero(),
            mouse_states: [State::Up; MOUSE_KEYS],
            keyboard_states: [State::Up; KEYBOARD_KEYS],
            type_buffer: String::with_capacity(10),

            prev_event_count: 0,
            event_source: event_source,
        }
    }

    /// Pulls new data from this input managers source. This should be called once per frame.
    pub fn refresh(&mut self) {
        self.mouse_delta = Vec2::zero(); 
        self.mouse_scroll = Vec2::zero();
        self.type_buffer.clear();

        for state in self.mouse_states.iter_mut() {
            if *state == State::Released { *state = State::Up; }
            if *state == State::Pressed  { *state = State::Down; }
        }
        for state in self.keyboard_states.iter_mut() {
            if *state == State::Released { *state = State::Up; }
            if *state == State::Pressed  { *state = State::Down; }
        } 

        self.prev_event_count = 0; 
        for event in self.event_source.try_iter() {
            self.prev_event_count += 1;

            match event {
                Event::MouseMoved(x, y) => {
                    let old_mouse_pos = self.mouse_pos;
                    self.mouse_pos = Vec2::new(x as f32, y as f32);
                    self.mouse_delta += self.mouse_pos - old_mouse_pos;
                },
                Event::MouseInput(state, button) => {
                    let index = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        MouseButton::Other(index) => index,
                    } as usize;
                    if index < self.mouse_states.len() {
                        self.mouse_states[index] = match state {
                            ElementState::Pressed => State::Pressed,
                            ElementState::Released => State::Released,
                        };
                    }
                },
                Event::MouseWheel(delta, _) => {
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            self.mouse_scroll += Vec2::new(x, y);
                        },
                        MouseScrollDelta::PixelDelta(x, y) => {
                            self.mouse_delta += Vec2::new(x, y);
                        },
                    }
                },
                Event::KeyboardInput(state, key, _name) => {
//                    if let Some(name) = name { println!("{:?} = 0x{:x}", name, key); }
                    let ref mut internal_state = self.keyboard_states[key as usize];
                    match state {
                        ElementState::Pressed => {
                            *internal_state = if internal_state.down() {
                                State::PressedRepeat
                            } else {
                                State::Pressed
                            }
                        },
                        ElementState::Released => *internal_state = State::Released,
                    };
                },
                Event::ReceivedCharacter(c) => self.type_buffer.push(c),
                _ => {},
            }
        } 
    }
    
    // Getters
    /// Position of the mouse cursor in pixels, relative to the top left corner of the screen
    pub fn mouse_pos(&self)   -> Vec2<f32> { self.mouse_pos }
    /// The distance the mouse cursor moved in the last frame
    pub fn mouse_delta(&self) -> Vec2<f32> { self.mouse_delta }
    /// The number of units scrolled in the last frame
    pub fn mouse_scroll(&self) -> f32 { self.mouse_scroll.y }
    /// The state of the given mouse key. Panics if `key` is greater than 4. The left
    /// mouse key is 0, the right key is 1 and the middle key is 2.
    pub fn mouse_key(&self, key: u8) -> State {
        let key = key as usize;
        if key >= self.mouse_states.len() {
            panic!("{} is not a valid index for a mouse key");
        }
        self.mouse_states[key]
    }
    /// The state of the given keyboard key. Note that `Key` represent scancodes.
    /// See [`Key`](enum.Key.html) for more info
    pub fn key(&self, key: Key) -> State {
        self.keyboard_states[key as usize]
    }
    /// Characters that have been typed. This is cleared each frame.
    pub fn typed(&self) -> &str {
        &self.type_buffer
    }

    /// The total number of events that occured between the last two calls to [`refresh`].
    ///
    /// [`refresh`]: struct.InputManager.html#fn.refresh
    pub fn prev_event_count(&self) -> usize {
        self.prev_event_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// The button is not held down.
    Up,
    /// The button is being held down. In the previous frame it was not held down.
    Pressed, 
    /// The button is being held down, and its repeat action triggered
    PressedRepeat,
    /// The button is being held down.
    Down,
    /// The button is not being held down. In the previous frame it was held down.
    Released,
}
impl State {
    /// Returns true if the button is being held down (`Down` or `Pressed`) and
    /// false otherwise (`Up`, `Released` or `PressedRepeat`).
    pub fn down(self) -> bool {
        match self {
            State::Up | State::Released => false,
            State::Down | State::Pressed | State::PressedRepeat => true,
        }
    }
    /// Returns true if the button is not being held down (`Up` or `Released`) and
    /// false otherwise (`Down` or `Pressed`).
    pub fn up(self) -> bool {
        !self.down()
    }
    /// Returns true if the button is being held down, but was not held down in the last
    /// frame (`Pressed`)
    pub fn pressed(self) -> bool { self == State::Pressed }
    /// Returns true either if this button is being held down and was not held down in the
    /// last frame (`Pressed`), or if the repeat action has been triggered by the key being
    /// held down for an extended amount of time (`PressedRepeat`).
    pub fn pressed_repeat(self) -> bool {
        self == State::Pressed || self == State::PressedRepeat
    }
    /// Returns true if the button is being not held down, but was held down in the last
    /// frame (`Released`)
    pub fn released(self) -> bool { self == State::Released }
}

/// Codes for most keys. Note that these are scancodes, so they refer to a position
/// on the keyboard, rather than a specific symbol. These can be used as parameters
/// to [`InputManager::key`](struct.InputManager.html#method.key). The names are
/// based on the american keyboard layout.
#[repr(u8)]
pub enum Key {
    Key1 = 0xa, Key2 = 0xb, Key3 = 0xc, Key4 = 0xd, Key5 = 0xe, Key6 = 0xf, Key7 = 0x10, Key8 = 0x11, Key9 = 0x12, Key0 = 0x13,
    Q = 0x18, W = 0x19, E = 0x1a, R = 0x1b, T = 0x1c, Y = 0x1d, U = 0x1e, I = 0x1f, P = 0x20,
    A = 0x26, S = 0x27, D = 0x28, F = 0x29, G = 0x2a, H = 0x2b, J = 0x2c, K = 0x2d, L = 0x2e,
    Z = 0x34, X = 0x35, C = 0x36, V = 0x37, B = 0x38, N = 0x39, M = 0x3a,
    Space = 0x41,

    Escape = 0x9, Grave  = 0x31, Tab = 0x17, CapsLock  = 0x42,
    LShift = 0x32, LCtrl = 0x25, LAlt = 0x40,
    RAlt  = 0x6c, RMeta  = 0x86, RCtrl = 0x69, RShift = 0x3e, Return = 0x24, Back = 0x16,

    Right = 0x72, Left = 0x71, Down = 0x74, Up = 0x6f,

    Insert = 0x76, Delete = 0x77, Home = 0x6e, End = 0x73, PageUp = 0x70, PageDown = 0x75,

    F1 = 0x43, F2 = 0x44, F3 = 0x45, F4 = 0x46, F5 = 0x47, F6 = 0x48, 
    F7 = 0x49, F8 = 0x4a, F9 = 0x4b, F10 = 0x4c, F11 = 0x5f, F12 = 0x60,
}


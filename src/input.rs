
use cable_math::Vec2;
use glutin::*;

const MOUSE_KEYS: usize = 5;
const KEYBOARD_KEYS: usize = 256; // This MUST be `u8::max_value() + 1`

pub struct InputManager {
    mouse_pos: Vec2<f32>,
    mouse_delta: Vec2<f32>,
    mouse_states: [State; MOUSE_KEYS],
    keyboard_states: [State; KEYBOARD_KEYS],
    type_buffer: String,
}

impl InputManager {
    pub fn new() -> InputManager {
        InputManager {
            mouse_pos: Vec2::zero(),
            mouse_delta: Vec2::zero(),
            mouse_states: [State::Up; MOUSE_KEYS],
            keyboard_states: [State::Up; KEYBOARD_KEYS],
            type_buffer: String::with_capacity(10),
        }
    }

    pub fn update(&mut self) {
        self.mouse_delta = Vec2::zero();

        for state in self.mouse_states.iter_mut() {
            if *state == State::Released { *state = State::Up; }
            if *state == State::Pressed  { *state = State::Down; }
        }
        for state in self.keyboard_states.iter_mut() {
            if *state == State::Released { *state = State::Up; }
            if *state == State::Pressed  { *state = State::Down; }
        }

        self.type_buffer.clear();
    }
    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::MouseMoved(x, y) => {
                let old_mouse_pos = self.mouse_pos;
                self.mouse_pos = Vec2::new(x as f32, y as f32);
                self.mouse_delta = self.mouse_pos - old_mouse_pos;
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
            Event::KeyboardInput(state, key, _) => {
                self.keyboard_states[key as usize] = match state {
                    ElementState::Pressed => State::Pressed,
                    ElementState::Released => State::Released,
                };
            },
            Event::ReceivedCharacter(c) => self.type_buffer.push(c),
            _ => {},
        }
    }
    
    // Getters
    /// Position of the mouse cursor in pixels, relative to the top left corner of the screen
    pub fn mouse_pos(&self)   -> Vec2<f32> { self.mouse_pos }
    /// The distance the mouse cursor moved in the last frame
    pub fn mouse_delta(&self) -> Vec2<f32> { self.mouse_delta }
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
    pub fn typed(&self) -> String {
        self.type_buffer.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// The button is not held down.
    Up,
    /// The button is beeing held down. In the previous frame it was not held down.
    Pressed, 
    /// The button is beeing held down.
    Down,
    /// The button is not beeing held down. In the previous frame it was held down.
    Released,
}
impl State {
    /// Returns true if the button is beeing held down (`Down` or `Pressed`) and
    /// false otherwise (`Up` or `Released`).
    pub fn is_down(self) -> bool {
        match self {
            State::Up | State::Released => false,
            State::Down | State::Pressed => true,
        }
    }
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

    Escape = 0x9, Grave  = 0x31, Tab = 0x17, CapsLock  = 0x42,
    LShift = 0x32, LCtrl = 0x25, LAlt = 0x40,
    RAlt  = 0x6c, RMeta  = 0x86, RCtrl = 0x69, RShift = 0x3e, Return = 0x24, Back = 0x16,
}


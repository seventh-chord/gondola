
//! Provides utilities for tracking the state of various input devices

use cable_math::Vec2;

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
    pub(crate) mouse_pos: Vec2<f32>,
    pub(crate) mouse_delta: Vec2<f32>,
    pub(crate) raw_mouse_delta: Vec2<f32>,
    pub(crate) mouse_scroll: f32,
    pub(crate) mouse_states: [KeyState; MOUSE_KEYS],
    pub(crate) keyboard_states: [KeyState; KEYBOARD_KEYS],
    pub(crate) type_buffer: String,
    pub(crate) focused: bool, 

    pub(crate) changed: bool, 
}

impl InputManager {
    /// Creates a new input manager. Passed to `Window::poll_events` each frame to get updated.
    pub fn new() -> InputManager {
        InputManager {
            mouse_pos: Vec2::zero(),
            mouse_delta: Vec2::zero(),
            raw_mouse_delta: Vec2::zero(),
            mouse_scroll: 0.0,
            mouse_states: [KeyState::Up; MOUSE_KEYS],
            keyboard_states: [KeyState::Up; KEYBOARD_KEYS],
            type_buffer: String::with_capacity(10),
            focused: false,

            changed: false,
        }
    }

    // Called by `Window::poll_events` in the platform layer
    pub(crate) fn refresh(&mut self) {
        self.mouse_delta = Vec2::zero(); 
        self.raw_mouse_delta = Vec2::zero(); 
        self.mouse_scroll = 0.0;
        self.type_buffer.clear();

        for state in self.mouse_states.iter_mut() {
            if *state == KeyState::Released       { *state = KeyState::Up; }
            if *state == KeyState::Pressed        { *state = KeyState::Down; }
            if *state == KeyState::PressedRepeat  { *state = KeyState::Down; }
        }
        for state in self.keyboard_states.iter_mut() {
            if *state == KeyState::Released       { *state = KeyState::Up; }
            if *state == KeyState::Pressed        { *state = KeyState::Down; }
            if *state == KeyState::PressedRepeat  { *state = KeyState::Down; }
        }

        self.changed = false; 
    }
    
    /// Position of the mouse cursor in pixels, relative to the top left corner of the screen
    pub fn mouse_pos(&self)   -> Vec2<f32> { self.mouse_pos }

    /// The distance on screen the mouse cursor moved in the last frame
    pub fn mouse_delta(&self) -> Vec2<f32> { self.mouse_delta }

    /// The distance the cursor moved, in unspecified units. This is the value reported directly
    /// from the mouse, without software acceleartion and simillar. This value should be used for
    /// e.g. first person cameras.
    pub fn raw_mouse_delta(&self) -> Vec2<f32> { self.raw_mouse_delta }

    /// The number of units scrolled in the last frame
    pub fn mouse_scroll(&self) -> f32 { self.mouse_scroll }

    /// The state of the given mouse key. Panics if `key` is greater than 4. The left
    /// mouse key is 0, the right key is 1 and the middle key is 2.
    pub fn mouse_key(&self, key: u8) -> KeyState {
        let key = key as usize;
        if key >= self.mouse_states.len() {
            panic!("{} is not a valid index for a mouse key");
        }
        self.mouse_states[key]
    }
    /// The state of the given keyboard key. Note that `Key` represent scancodes.
    /// See [`Key`](enum.Key.html) for more info
    pub fn key(&self, key: Key) -> KeyState {
        self.keyboard_states[key as usize]
    }
    
    /// Characters that have been typed. This is cleared each frame.
    pub fn typed(&self) -> &str {
        &self.type_buffer
    }

    /// True if new events have been added in the last poll.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// True if the window is receiving keyboard input
    pub fn focused(&self) -> bool {
        self.focused
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyState {
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

impl KeyState {
    /// Returns true if the button is being held down (`Down` or `Pressed`) and
    /// false otherwise (`Up`, `Released` or `PressedRepeat`).
    pub fn down(self) -> bool {
        match self {
            KeyState::Up | KeyState::Released => false,
            KeyState::Down | KeyState::Pressed | KeyState::PressedRepeat => true,
        }
    }
    /// Returns true if the button is not being held down (`Up` or `Released`) and
    /// false otherwise (`Down` or `Pressed`).
    pub fn up(self) -> bool {
        !self.down()
    }
    /// Returns true if the button is being held down, but was not held down in the last
    /// frame (`Pressed`)
    pub fn pressed(self) -> bool { self == KeyState::Pressed }
    /// Returns true either if this button is being held down and was not held down in the
    /// last frame (`Pressed`), or if the repeat action has been triggered by the key being
    /// held down for an extended amount of time (`PressedRepeat`).
    pub fn pressed_repeat(self) -> bool {
        self == KeyState::Pressed || self == KeyState::PressedRepeat
    }
    /// Returns true if the button is being not held down, but was held down in the last
    /// frame (`Released`)
    pub fn released(self) -> bool { self == KeyState::Released }
}

/// Codes for most keys. Note that these are scancodes, so they refer to a position
/// on the keyboard, rather than a specific symbol. These can be used as parameters
/// to [`InputManager::key`](struct.InputManager.html#method.key). The names are
/// based on the american keyboard layout.
///
/// Scancodes are target specific, so the values asigned to each enum name might vary from platform
/// to platform. On some platforms not all keys are available. Check the source code for more
/// detailed information on this.
#[derive(Debug, Copy, Clone)]
#[cfg(target_os = "linux")]
#[repr(u8)]
pub enum Key {
    Key1 = 0xa, Key2 = 0xb, Key3 = 0xc, Key4 = 0xd, Key5 = 0xe, 
    Key6 = 0xf, Key7 = 0x10, Key8 = 0x11, Key9 = 0x12, Key0 = 0x13,

    Q = 0x18, W = 0x19, E = 0x1a, R = 0x1b, T = 0x1c, Y = 0x1d, U = 0x1e, I = 0x1f, O = 0x20, P = 0x21,
    A = 0x26, S = 0x27, D = 0x28, F = 0x29, G = 0x2a, H = 0x2b, J = 0x2c, K = 0x2d, L = 0x2e,
    Z = 0x34, X = 0x35, C = 0x36, V = 0x37, B = 0x38, N = 0x39, M = 0x3a,

    Space = 0x41,

    Escape = 0x9, Grave  = 0x31, Tab = 0x17, CapsLock  = 0x42,
    LShift = 0x32, LCtrl = 0x25, LAlt = 0x40,
    RAlt  = 0x6c, RMeta  = 0x86, RCtrl = 0x69, RShift = 0x3e, Return = 0x24, Back = 0x16,

    Right = 0x72, Left = 0x71, Down = 0x74, Up = 0x6f,

    Insert = 0x76, Delete = 0x77, Home = 0x6e, End = 0x73, PageUp = 0x70, PageDown = 0x75,

    F1 = 0x43, F2 = 0x44, F3 = 0x45, F4 = 0x46,  F5 = 0x47,  F6 = 0x48, 
    F7 = 0x49, F8 = 0x4a, F9 = 0x4b, F10 = 0x4c, F11 = 0x5f, F12 = 0x60,
}

/// Codes for most keys. Note that these are scancodes, so they refer to a position on the
/// keyboard, rather than a specific symbol. These can be used as parameters to
/// [`InputManager::key`](struct.InputManager.html#method.key). The names are based on the american
/// keyboard layout.  
///
/// Scancodes are target specific, so the values asigned to each enum name might vary from platform
/// to platform. On some platforms not all keys are available. Check the source code for more
/// detailed information on this.
#[derive(Debug, Copy, Clone)]
#[cfg(target_os = "windows")]
#[repr(u8)]
pub enum Key {
    Key1 = 0x2, Key2 = 0x3, Key3 = 0x4, Key4 = 0x5, Key5 = 0x6,
    Key6 = 0x7, Key7 = 0x8, Key8 = 0x9, Key9 = 0xa, Key0 = 0xb,

    Q = 0x10, W = 0x11, E = 0x12, R = 0x13, T = 0x14, Y = 0x15, U = 0x16, I = 0x17, O = 0x18, P = 0x19,
    A = 0x1e, S = 0x1f, D = 0x20, F = 0x21, G = 0x22, H = 0x23, J = 0x24, K = 0x25, L = 0x26, 
    Z = 0x2c, X = 0x2d, C = 0x2e, V = 0x2f, B = 0x30, N = 0x31, M = 0x32,

    Space = 0x39,

    Escape = 0x1, 
//    Grave  = 0x31, 
    Tab = 0xf,
//    CapsLock  = 0x42, 
    LShift = 0x2a,
    LCtrl = 0x1d,
//    LAlt = 0x40,
//    RAlt  = 0x6c,
//    RMeta  = 0x86,
//    RCtrl = 0x1d, // Same scancode as LCtrl :/
    RShift = 0x36,
    Return = 0x1c,
    Back = 0xe,

    Right = 0x4d, Left = 0x4b, Down = 0x50, Up = 0x48,

    Insert = 0x52, Delete = 0x53, Home = 0x47, End = 0x4f, PageUp = 0x49, PageDown = 0x51,

    F1 = 0x3b, F2 = 0x3c, F3 = 0x3d, F4 = 0x3e,  F5 = 0x3f,  F6 = 0x40,
    F7 = 0x41, F8 = 0x42, F9 = 0x43, F10 = 0x44, F11 = 0x57, F12 = 0x58,
}


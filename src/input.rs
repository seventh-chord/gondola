
//! Provides utilities for tracking the state of various input devices

use cable_math::Vec2;

const MOUSE_KEYS: usize = 5;
const KEYBOARD_KEYS: usize = 256; // This MUST be `u8::max_value() + 1`

/// Passed to `Window::poll_events` each frame to get updated.
#[derive(Clone)]
pub struct Input {
    /// The position of the mouse, in window space
    pub mouse_pos: Vec2<f32>,
    /// The amount `mouse_pos` has changed since last frame
    /// TODO document whether this stays constant if the mouse is grabbed
    pub mouse_delta: Vec2<f32>,

    /// The amount of movement directly reported by the mouse sensor. Should be used for e.g. first
    /// person cameras in games.
    pub raw_mouse_delta: Vec2<f32>,

    /// Units scrolled in the last frame. 1.0 corresponds to one tick of the wheel
    pub mouse_scroll: f32,

    /// The state of mouse keys. 0 is left, 1 is right, 2 is middle. 3 and 4 are usually the keys
    /// for clicking the mousewheel laterally, for mice that have such keys. Sometimes they are
    /// also placed on the side of the mouse.
    ///
    /// On linux, 3 and 4 are always `Up`, because these codes are used for the scroll wheel
    /// internally.
    pub mouse_keys: [KeyState; MOUSE_KEYS],

    /// The state of keyboard keys. Can also be accessed more ergonomically throug the
    /// `Input::key()` method
    pub keys: [KeyState; KEYBOARD_KEYS],

    /// Cleared each frame. Contains typed characters in the order they where typed
    pub type_buffer: String,

    pub window_has_keyboard_focus: bool, 
    pub received_events_this_frame: bool, 
}

impl Input {
    pub fn new() -> Input {
        Input {
            mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            raw_mouse_delta: Vec2::ZERO,
            mouse_scroll: 0.0,
            mouse_keys: [KeyState::Up; MOUSE_KEYS],
            keys: [KeyState::Up; KEYBOARD_KEYS],
            type_buffer: String::with_capacity(10),
            window_has_keyboard_focus: false,
            received_events_this_frame: false,
        }
    }

    // Called by `Window::poll_events` in the platform layer
    pub(crate) fn refresh(&mut self) {
        self.mouse_delta = Vec2::ZERO; 
        self.raw_mouse_delta = Vec2::ZERO; 
        self.mouse_scroll = 0.0;
        self.type_buffer.clear();

        for state in self.mouse_keys.iter_mut() {
            if *state == KeyState::Released       { *state = KeyState::Up; }
            if *state == KeyState::Pressed        { *state = KeyState::Down; }
            if *state == KeyState::PressedRepeat  { *state = KeyState::Down; }
        }

        for state in self.keys.iter_mut() {
            if *state == KeyState::Released       { *state = KeyState::Up; }
            if *state == KeyState::Pressed        { *state = KeyState::Down; }
            if *state == KeyState::PressedRepeat  { *state = KeyState::Down; }
        }

        self.received_events_this_frame = false; 
    }
    
    /// The state of the given keyboard key. Note that `Key` represent scancodes.
    /// See [`Key`](enum.Key.html) for more info
    pub fn key(&self, key: Key) -> KeyState {
        self.keys[key as usize]
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


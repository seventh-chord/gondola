
//! A semi-safe, semi-stateless wrapper around OpenGL 3.3 Core. This library provides various
//! utilities to make using OpenGL 3.3 safer. It uses rust's type system to encode some information
//! which helps prevent common errors. This library is primarily intended to be used for games,
//! but you can also use it to create other graphics applications.
//!
//! # Example
//! Note: A more complete example can be found in `src/bin/window_demo.rs`.
//!
//! ```rust,no_run
//! use gondola::{Window, WindowCommon, InputManager};
//!
//! let mut input = InputManager::new();
//! let mut window = Window::new("My title");
//!
//! while !window.close_requested {
//!     window.poll_events(input);
//!
//!     // Update and render here!
//!
//!     window.swap_buffers();
//! }
//! ```

#[cfg(feature = "serialize")]
extern crate serde;

extern crate gl;
extern crate png;
extern crate rusttype;

extern crate cable_math;

mod color;
mod input;
mod window;
mod time;
mod region;

pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod graphics;
pub mod framebuffer;
pub mod font;
pub mod draw_group;
//pub mod ui; // Temporarily disabled. Broken due to changes in font code. Should be rewritten to use draw_group

pub use color::*;
pub use input::*;
pub use window::*;
pub use time::*;
pub use region::*;
pub use draw_group::DrawGroup;

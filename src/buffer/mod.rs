
//! Utilities for storing and drawing data in GPU buffers.

const DEFAULT_SIZE: usize = 100;

mod primitives;
mod vertex_buffer;
mod primitive_buffer;

pub use self::primitives::*;
pub use self::vertex_buffer::*;
pub use self::primitive_buffer::*;


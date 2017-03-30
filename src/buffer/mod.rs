
//! Utilities for storing and drawing data in GPU buffers.
//!
//! This module defines four primary structs for storing data:
//!
//!  - [`VertexBuffer`] is the simplest to use type, and you probably want to use it in most cases.
//!    To use it you define a custom type which implements [`Vertex`]. You can then store a slice of
//!    your custom vertex type in a buffer and draw it using any [`PrimitiveMode`] you like.
//!  - [`IndexedVertexBuffer`] works similarly to [`VertexBuffer`], but it allows you to specify a
//!    additional index buffer, which is handy when many primitives reuse the same vertices.
//!  - [`PrimitiveBuffer`] is a direct wrapper around a OpenGL buffer object. It allows you to
//!    store any type which implements [`VertexData`] in a graphics buffer. Primitive buffers are
//!    used when you need low level control over how data is managed, or when you want to do
//!    something not exposed through vertex buffers. 
//!  - [`VertexArray`] is used to specify how data in a primitive buffer is passed to a shader. You
//!    usually want to use a [`VertexBuffer`], which automatically manages primitive buffers and
//!    vertex arrays for you.
//!
//! [`VertexBuffer`]:           struct.VertexBuffer.html
//! [`IndexedVertexBuffer`]:    struct.IndexedVertexBuffer.html
//! [`PrimitiveBuffer`]:        struct.PrimitiveBuffer.html
//! [`VertexArray`]:            struct.VertexArray.html
//! [`Vertex`]:                 trait.Vertex.html 
//! [`VertexData`]:             trait.VertexData.html 
//! [`PrimitiveMode`]:          enum.PrimitiveMode.html

const DEFAULT_SIZE: usize = 100;

mod primitives;
mod vertex_buffer;
mod primitive_buffer;

pub use self::primitives::*;
pub use self::vertex_buffer::*;
pub use self::primitive_buffer::*;


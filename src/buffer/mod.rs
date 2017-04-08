
//! Utilities for storing and drawing data in GPU buffers.
//!
//! This module defines five primary structs for storing data:
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
//!  - [`TextureBuffer`] is a primitives buffer which can be bound to a texture target. This allows you
//!    to access the data stored in it from glsl using a `samplerBuffer`.
//!  - [`VertexArray`] is used to specify how data in a primitive buffer is passed to a shader. You
//!    usually want to use a [`VertexBuffer`], which automatically manages primitive buffers and
//!    vertex arrays for you.
//!
//! [`VertexBuffer`]:           struct.VertexBuffer.html
//! [`IndexedVertexBuffer`]:    struct.IndexedVertexBuffer.html
//! [`TextureBuffer`]:          struct.TextureBuffer.html
//! [`PrimitiveBuffer`]:        struct.PrimitiveBuffer.html
//! [`VertexArray`]:            struct.VertexArray.html
//! [`Vertex`]:                 trait.Vertex.html 
//! [`VertexData`]:             trait.VertexData.html 
//! [`PrimitiveMode`]:          enum.PrimitiveMode.html

const DEFAULT_SIZE: usize = 100;

mod primitives;
mod vertex_buffer;
mod primitive_buffer;
mod texture_buffer;

pub use self::primitives::*;
pub use self::vertex_buffer::*;
pub use self::primitive_buffer::*;
pub use self::texture_buffer::*;

/// Reperesents the data needed for a call to `gl::EnableVertexAttribArray`,
/// `gl::VertexAttribPointer` and `gl::VertexAttribDivisor`. This is mainly
/// intended for internal usage and when deriving [`Vertex`].
///
/// [`Vertex`]: struct.Vertex.html
#[derive(Debug, Clone)]
pub struct AttribBinding {
    /// The vertex attribute to which this binding will serve values.
    pub index: usize,
    /// The number of primitives per vertex this attribute will serve to shaders.
    pub primitives: usize,
    /// The type of primitives which this attribute will serve to shaders. Should be a constant
    /// defined by OpenGL.
    pub primitive_type: u32,
    /// If set to true, integer types will be parsed as floats and mapped to the range `0.0..1.0`
    /// for unsigned integers and `-1.0..1.0` for signed integers.
    pub normalized: bool,
    /// The distance, in bytes, between each set of primitives
    pub stride: usize,
    /// The index, in bytes, of the first byte of data
    pub offset: usize,

    /// The number of vertices from other sources for which this source will be used. For example,
    /// if set to 3 every set of three vertices will use one instance from this source.
    pub divisor: usize,
}

impl AttribBinding {
    /// Calls `gl::EnableVertexAttribArray`, `gl::VertexAttribPointer` and `gl::VertexAttribDivisor`.
    pub fn enable(&self) {
        use gl;
        use gl::types::*;

        unsafe {
            gl::EnableVertexAttribArray(self.index as GLuint);
            gl::VertexAttribPointer(self.index as GLuint, self.primitives as GLint,
                                    self.primitive_type as GLenum, self.normalized as GLboolean,
                                    self.stride as GLsizei, self.offset as *const GLvoid);
            gl::VertexAttribDivisor(self.index as GLuint, self.divisor as GLuint);
        }
    }
}


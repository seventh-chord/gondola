
use super::*;
use gl;
use gl::types::*;
use std::ops::{Deref, DerefMut};

/// A [`PrimitiveBuffer`] which can be bound to a texture target and accessed from shaders. This
/// struct dereferences to [`PrimitiveBuffer`], so it can be used like a normal buffer when needed.
///
/// [`PrimitiveBuffer`]:           struct.PrimitiveBuffer.html
pub struct TextureBuffer<T: VertexData> {
    buffer: PrimitiveBuffer<T>,
    texture: GLuint,
}

impl<T: VertexData> TextureBuffer<T> {
    /// Creates a new texture buffer, preallocating space for 100 vertices.
    pub fn new(access_primitives: usize, usage: BufferUsage) -> TextureBuffer<T> {
        TextureBuffer::with_capacity(access_primitives, usage, DEFAULT_SIZE)
    }

    /// Creates a new texture buffer, preallocating space for the given number of vertices.
    pub fn with_capacity(access_primitives: usize, usage: BufferUsage, initial_capacity: usize) -> TextureBuffer<T> {
        let buffer = PrimitiveBuffer::with_capacity(BufferTarget::Array, usage, initial_capacity);
        TextureBuffer::from_buffer(access_primitives, buffer)
    }

    /// Creates a new texture buffer, storing the given vertices on the GPU.
    pub fn with_data(access_primitives: usize, data: &[T]) -> TextureBuffer<T> {
        let buffer = PrimitiveBuffer::with_data(BufferTarget::Array, data);
        TextureBuffer::from_buffer(access_primitives, buffer)
    }

    /// Converts a vertex buffer into a texture buffer. 
    ///
    /// `access_primitives` specifies the number of primitives that will be accessible per texel in 
    /// a shader. This must be between 1 and 4 (both inclusive), and `T::primitives()` must be 
    /// divisible by it. For example, if your vertex data has 10 primitives `access_primitives` can
    /// be 1 and 2.
    pub fn from_buffer(access_primitives: usize, buffer: PrimitiveBuffer<T>) -> TextureBuffer<T> {
        assert!(access_primitives > 0 && access_primitives <= 4, 
                "access_primitives ({}) must be equal to the number of primitives in a valid image format (R, RG, RGB or RGBA)",
                access_primitives);
        assert!(T::primitives() % access_primitives == 0,
                "T::primitives() ({}) must be divisible by access_primitives ({})",
                T::primitives(), access_primitives);

        let mut texture = 0;
        let format = match (T::Primitive::gl_enum(), access_primitives) {
            (gl::FLOAT, 1) => gl::R32F,
            (gl::FLOAT, 2) => gl::RG32F,
            (gl::FLOAT, 3) => gl::RGB32F,
            (gl::FLOAT, 4) => gl::RGBA32F,
            // I cant be bothered to implement other types as I probably never will use them. This
            // should be trivial to extend if you get a panic.
            _ => panic!("Invalid vertex data for texture buffer (access_primitives: {}, type: {})",
                        access_primitives, T::Primitive::rust_name() ),
        };

        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_BUFFER, texture);
            gl::TexBuffer(gl::TEXTURE_BUFFER, format, buffer.buffer);
        }

        TextureBuffer {
            buffer: buffer,
            texture: texture,
        }
    }

    /// Binds this buffer to the given texture unit. Note that this binds the texture to the
    /// `gl::TEXTURE_BUFFER` target.
    pub fn bind_texture(&self, unit: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit);
            gl::BindTexture(gl::TEXTURE_BUFFER, self.texture);
        }
    }
}

impl<T: VertexData> Deref for TextureBuffer<T> {
    type Target = PrimitiveBuffer<T>;
    fn deref(&self) -> &PrimitiveBuffer<T> {
        &self.buffer
    }
}
impl<T: VertexData> DerefMut for TextureBuffer<T> {
    fn deref_mut(&mut self) -> &mut PrimitiveBuffer<T> {
        &mut self.buffer
    }
}

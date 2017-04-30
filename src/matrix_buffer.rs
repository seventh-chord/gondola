
//! A utility to store matrices in a uniform buffer for access from shaders.

use gl::types::*;
use buffer::*;
use cable_math::Mat4;

/// A utility to store matrices in a uniform buffer for access from shaders.
pub struct MatrixBuffer {
    binding_index: GLuint,
    buffer: PrimitiveBuffer<Mat4<f32>>,
}

impl MatrixBuffer {
    pub fn new(binding_index: usize) -> MatrixBuffer {
        MatrixBuffer {
            binding_index: binding_index as GLuint,
            buffer: PrimitiveBuffer::new(BufferTarget::Uniform, BufferUsage::DynamicDraw),
        }
    }

    /// Writes
    pub fn store(&mut self, matrices: &[Mat4<f32>]) {
        self.buffer.put_at_start(matrices);
        self.buffer.bind_base(self.binding_index);
    }
}

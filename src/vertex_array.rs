
use gl;
use gl::types::*;
use graphics_buffer::GraphicsBuffer;
use std::ops::Range;

pub struct VertexArray {
    array: GLuint,
}

impl VertexArray {
    pub fn new() -> VertexArray {
        let mut array = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut array);
        }
        VertexArray {
            array: array,
        }
    }

    /// Adds a graphic buffer from which this vertex array will pull data when drawing
    pub fn add_data_source(&self, source: &GraphicsBuffer, index: usize, size: usize, stride: usize, offset: usize) {
        source.bind();

        unsafe {
            gl::BindVertexArray(self.array);
            gl::EnableVertexAttribArray(index as GLuint);

            let data_type = source.data_type();
            gl::VertexAttribPointer(
                index as GLuint, size as GLint,
                data_type.get_gl_enum(), false as GLboolean,
                (stride * data_type.size()) as GLsizei, (offset * data_type.size()) as *const GLvoid
            );
        }
    }

    /// Draws the given primitives with the graphics buffers bound to this vertex array 
    pub fn draw(&self, mode: PrimitiveMode, range: Range<usize>) {
        unsafe {
            gl::BindVertexArray(self.array);
            gl::DrawArrays(mode.get_gl_enum(), range.start as GLint, (range.end - range.start) as GLsizei);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.array);
        }
    }
}

#[derive(Copy, Clone)]
pub enum PrimitiveMode {
    Points, LineStrip, LineLoop, Lines, LineStripAdjacency, LinesAdjacency,
    TriangleStrip, TriangleFan, Triangles, TriangleStripAdjacency, TrianglesAdjacency,
} 

impl PrimitiveMode {
    fn get_gl_enum(&self) -> GLenum {
        match *self {
            PrimitiveMode::Points                  => gl::POINTS,
            PrimitiveMode::LineStrip               => gl::LINE_STRIP,
            PrimitiveMode::LineLoop                => gl::LINE_LOOP,
            PrimitiveMode::Lines                   => gl::LINES,
            PrimitiveMode::LineStripAdjacency      => gl::LINE_STRIP_ADJACENCY,
            PrimitiveMode::LinesAdjacency          => gl::LINES_ADJACENCY,
            PrimitiveMode::TriangleStrip           => gl::TRIANGLE_STRIP,
            PrimitiveMode::TriangleFan             => gl::TRIANGLE_FAN,
            PrimitiveMode::Triangles               => gl::TRIANGLES,
            PrimitiveMode::TriangleStripAdjacency  => gl::TRIANGLE_STRIP_ADJACENCY,
            PrimitiveMode::TrianglesAdjacency      => gl::TRIANGLES_ADJACENCY,
        }
    }
}


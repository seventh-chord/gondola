
use gl;
use gl::types::*;
use primitive_buffer::PrimitiveBuffer;
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
    pub fn add_data_source(&self, source: &PrimitiveBuffer, index: usize, size: usize, stride: usize, offset: usize) {
        source.bind();

        unsafe {
            gl::BindVertexArray(self.array);
            gl::EnableVertexAttribArray(index as GLuint);

            let data_type = source.data_type();
            gl::VertexAttribPointer(
                index as GLuint, size as GLint,
                data_type as GLenum, false as GLboolean,
                (stride * data_type.size()) as GLsizei, (offset * data_type.size()) as *const GLvoid
            );
        }
    }

    /// Draws the given primitives with the graphics buffers bound to this vertex array 
    pub fn draw(&self, mode: PrimitiveMode, range: Range<usize>) {
        unsafe {
            gl::BindVertexArray(self.array);
            gl::DrawArrays(mode as GLenum, range.start as GLint, (range.end - range.start) as GLsizei);
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
    Points                      = gl::POINTS as isize,
    LineStrip                   = gl::LINE_STRIP as isize,
    LineLoop                    = gl::LINE_LOOP as isize,
    Lines                       = gl::LINES as isize,
    LineStripAdjacency          = gl::LINE_STRIP_ADJACENCY as isize,
    LinesAdjacency              = gl::LINES_ADJACENCY as isize,
    TriangleStrip               = gl::TRIANGLE_STRIP as isize,
    TriangleFan                 = gl::TRIANGLE_FAN as isize,
    Triangles                   = gl::TRIANGLES as isize,
    TriangleStripAdjacency      = gl::TRIANGLE_STRIP_ADJACENCY as isize,
    TrianglesAdjacency          = gl::TRIANGLES_ADJACENCY as isize,
} 

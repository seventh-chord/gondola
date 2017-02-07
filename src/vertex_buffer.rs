
use gl;
use gl::types::*;
use std;
use primitive_buffer::{BufferUsage, BufferTarget};
use vertex_array::PrimitiveMode;

const DEFAULT_SIZE: usize = 100;

pub trait Vertex {
    fn bytes_per_vertex() -> usize;
    fn setup_attrib_pointers();
}

/// A GPU buffer which holds a list of verticies for rendering.
pub struct VertexBuffer<A: Vertex> {
    // We are generic over the vertex type, but dont actually store any vertices
    phantom: std::marker::PhantomData<A>,

    vertex_count: usize,
    allocated: usize,

    primitive_mode: PrimitiveMode,
    usage: BufferUsage,

    vbo: GLuint,
    vao: GLuint
}

impl <A: Vertex> VertexBuffer<A> {
    /// Creates a new vertex buffer, prealocating space for 100 vertices.
    pub fn new(primitive_mode: PrimitiveMode, usage: BufferUsage) -> VertexBuffer<A> {
        let vertices = DEFAULT_SIZE;
        let bytes = A::bytes_per_vertex() * vertices;

        let mut vbo = 0;
        let mut vao = 0;

        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(BufferTarget::ArrayBuffer as GLenum, vbo);
            gl::BufferData(BufferTarget::ArrayBuffer as GLenum, bytes as GLsizeiptr, std::ptr::null(), usage as GLenum);

            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            A::setup_attrib_pointers();
        }

        VertexBuffer {
            phantom: std::marker::PhantomData,

            vertex_count: 0,
            allocated: vertices,

            primitive_mode: primitive_mode,
            usage: usage,

            vbo: vbo,
            vao: vao,
        }
    }

    /// Creates a new vertex buffer, storing the given vertices on the GPU.
    pub fn from_data(primitive_mode: PrimitiveMode, data: &[A]) -> VertexBuffer<A> {
        let vertices = data.len();
        let bytes = A::bytes_per_vertex() * vertices;

        let mut vbo = 0;
        let mut vao = 0;

        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(BufferTarget::ArrayBuffer as GLenum, vbo);
            gl::BufferData(
                BufferTarget::ArrayBuffer as GLenum,
                bytes as GLsizeiptr,
                std::mem::transmute(&data[0]),
                BufferUsage::StaticDraw as GLenum
            );

            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            A::setup_attrib_pointers();
        }

        VertexBuffer {
            phantom: std::marker::PhantomData,

            vertex_count: data.len(),
            allocated: data.len(),

            primitive_mode: primitive_mode,
            usage: BufferUsage::StaticDraw,

            vbo: vbo,
            vao: vao,
        }
    }

    /// Puts the given vertices at the start of this buffer, replacing any vertices
    /// which where previously in that location. This resizes the underlying buffer
    /// if more space is needed to store the new data.
    pub fn put_at_start(&mut self, data: &[A]) {
        self.put(0, data);
    }
    /// Puts the given vertices at the end of this buffer, behind any data which is
    /// allready in it. This resizes the underlying buffer if more space is needed
    /// to store the new data.
    pub fn put_at_end(&mut self, data: &[A]) {
        let vertex_count = self.vertex_count;
        self.put(vertex_count, data);
    }
    /// Puts the given vertices at the given index in this buffer, overwriting any
    /// vertices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new data.
    pub fn put(&mut self, index: usize, data: &[A]) {
        let start = index;
        let end = index + data.len();
        let bytes = data.len() * A::bytes_per_vertex();

        if end > self.allocated {
            self.allocate(end); // Maybe we should allocate some extra space
        }

        unsafe {
            gl::BindBuffer(BufferTarget::ArrayBuffer as GLenum, self.vbo);
            gl::BufferSubData(
                BufferTarget::ArrayBuffer as GLenum,
                (start * A::bytes_per_vertex()) as GLintptr,
                (data.len() * A::bytes_per_vertex()) as GLsizeiptr,
                std::mem::transmute(&data[0])
            );
        }
    }

    /// The number of vertices that are stored in GPU memory.
    pub fn len(&self) -> usize {
        self.vertex_count
    }

    /// The number of vertices that can be stored in this buffer without
    /// realocating memory. 
    pub fn capacity(&self) -> usize {
        self.allocated
    }

    /// Sets the number of vertices that can be stored in this buffer without
    /// realocating memory.
    pub fn allocate(&mut self, new_size: usize) {
        // Only realocate if necessary
        if new_size > self.allocated {
            let mut new_vbo = 0;
            let bytes = new_size * A::bytes_per_vertex();

            unsafe {
                gl::GenBuffers(1, &mut new_vbo);
                gl::BindBuffer(BufferTarget::ArrayBuffer as GLenum, new_vbo);
                gl::BufferData(BufferTarget::ArrayBuffer as GLenum, bytes as GLsizeiptr, std::ptr::null(), self.usage as GLenum);

                gl::BindVertexArray(self.vao);
                A::setup_attrib_pointers();

                // Copy old data
                gl::BindBuffer(BufferTarget::CopyReadBuffer as GLenum, self.vbo);
                gl::CopyBufferSubData(
                    BufferTarget::CopyReadBuffer as GLenum,
                    BufferTarget::ArrayBuffer as GLenum,
                    0, 0,
                    (self.vertex_count * A::bytes_per_vertex()) as GLsizeiptr
                );
                gl::DeleteBuffers(1, &mut self.vbo);
            }

            self.vbo = new_vbo;
            self.allocated = new_size
        }
    }

    /// Draws the contents of this vertex buffer with the primitive mode specified
    /// at construction.
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(self.primitive_mode as GLenum, 0, self.vertex_count as GLsizei);
        }
    }
}

impl <A: Vertex> Drop for VertexBuffer<A> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
            gl::DeleteVertexArrays(1, &mut self.vao);
        }
    }
}


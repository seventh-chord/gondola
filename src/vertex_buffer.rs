
use gl;
use gl::types::*;
use std;
use primitive_buffer::{BufferUsage, BufferTarget};
use vertex_array::PrimitiveMode;

const DEFAULT_SIZE: usize = 100;

pub struct VertexBuffer<A: Vertex> {
    // We are generic over the vertex type, but dont actually store any vertices
    phantom: std::marker::PhantomData<A>,

    vertex_count: usize,
    allocated_vertices: usize,

    primitive_mode: PrimitiveMode,
    usage: BufferUsage,

    vbo: GLuint,
    vao: GLuint
}

impl <A: Vertex> VertexBuffer<A> {
    /// Creates a new vertex buffer, prealocating space for 100 vertices
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
            allocated_vertices: vertices,

            primitive_mode: primitive_mode,
            usage: usage,

            vbo: vbo,
            vao: vao,
        }
    }

    /// Creates a new vertex buffer, storing the given vertices on the GPU
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
            allocated_vertices: data.len(),

            primitive_mode: primitive_mode,
            usage: BufferUsage::StaticDraw,

            vbo: vbo,
            vao: vao,
        }
    }

    /// The number of vertices that are stored in GPU memory
    pub fn len(&self) -> usize {
        self.vertex_count
    }

    /// The number of vertices that can be stored in GPU memory without realocating space
    pub fn allocated(&self) -> usize {
        self.allocated_vertices
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(self.primitive_mode as GLenum, 0, self.vertex_count as GLsizei);
        }
    }
}

pub trait Vertex {
    fn bytes_per_vertex() -> usize;
    fn setup_attrib_pointers();
}


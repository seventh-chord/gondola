
use gl;
use gl::types::*;
use std;
use primitive_buffer::{BufferUsage, BufferTarget};
use vertex_array::PrimitiveMode;

const DEFAULT_SIZE: usize = 100;

/// A GPU buffer which holds a list of verticies for rendering.
pub struct VertexBuffer<T: Vertex> {
    // We are generic over the vertex type, but dont actually store any vertices
    phantom: std::marker::PhantomData<T>,

    vertex_count: usize,
    allocated: usize,

    primitive_mode: PrimitiveMode,
    usage: BufferUsage,

    vbo: GLuint,
    vao: GLuint
}

impl <T: Vertex> VertexBuffer<T> {
    /// Creates a new vertex buffer, prealocating space for 100 vertices.
    pub fn new(primitive_mode: PrimitiveMode, usage: BufferUsage) -> VertexBuffer<T> {
        let vertices = DEFAULT_SIZE;
        let bytes = T::bytes_per_vertex() * vertices;

        let mut vbo = 0;
        let mut vao = 0;

        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(BufferTarget::Array as GLenum, vbo);
            gl::BufferData(BufferTarget::Array as GLenum, bytes as GLsizeiptr, std::ptr::null(), usage as GLenum);

            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            T::setup_attrib_pointers();
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
    pub fn from_data(primitive_mode: PrimitiveMode, data: &[T]) -> VertexBuffer<T> {
        let vertices = data.len();
        let bytes = T::bytes_per_vertex() * vertices;

        let mut vbo = 0;
        let mut vao = 0;

        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(BufferTarget::Array as GLenum, vbo);
            gl::BufferData(
                BufferTarget::Array as GLenum,
                bytes as GLsizeiptr,
                std::mem::transmute(&data[0]),
                BufferUsage::StaticDraw as GLenum
            );

            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            T::setup_attrib_pointers();
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
    pub fn put_at_start(&mut self, data: &[T]) {
        self.put(0, data);
    }
    /// Puts the given vertices at the end of this buffer, behind any data which is
    /// allready in it. This resizes the underlying buffer if more space is needed
    /// to store the new data.
    pub fn put_at_end(&mut self, data: &[T]) {
        let vertex_count = self.vertex_count;
        self.put(vertex_count, data);
    }
    /// Puts the given vertices at the given index in this buffer, overwriting any
    /// vertices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new data.
    pub fn put(&mut self, index: usize, data: &[T]) {
        let start = index;
        let end = index + data.len();

        if end > self.allocated {
            self.allocate(end); // Maybe we should allocate some extra space
        }

        if end > self.vertex_count {
            self.vertex_count = end;
        }

        unsafe {
            gl::BindBuffer(BufferTarget::Array as GLenum, self.vbo);
            gl::BufferSubData(
                BufferTarget::Array as GLenum,
                (start * T::bytes_per_vertex()) as GLintptr,
                (data.len() * T::bytes_per_vertex()) as GLsizeiptr,
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
            let bytes = new_size * T::bytes_per_vertex();

            unsafe {
                gl::GenBuffers(1, &mut new_vbo);
                gl::BindBuffer(BufferTarget::Array as GLenum, new_vbo);
                gl::BufferData(BufferTarget::Array as GLenum, bytes as GLsizeiptr, std::ptr::null(), self.usage as GLenum);

                gl::BindVertexArray(self.vao);
                T::setup_attrib_pointers();

                // Copy old data
                gl::BindBuffer(BufferTarget::CopyRead as GLenum, self.vbo);
                gl::CopyBufferSubData(
                    BufferTarget::CopyRead as GLenum,
                    BufferTarget::Array as GLenum,
                    0, 0,
                    (self.vertex_count * T::bytes_per_vertex()) as GLsizeiptr
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

impl <T: Vertex> Drop for VertexBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
            gl::DeleteVertexArrays(1, &mut self.vao);
        }
    }
}

/// Verterx buffers store a list of `Vertex`es (called vertices in proper
/// english) on the GPU
pub trait Vertex {
    fn bytes_per_vertex() -> usize;
    fn setup_attrib_pointers();
    fn gen_shader_input_decl() -> String;
}

/// All fields of a struct need to implement this in order for #[derive(Vertex)]
/// to work. Implemented for single fields and up to four length touples of the
/// types `f32`, `i32`, `u32`
pub trait VertexComponent {
    fn bytes() -> usize;
    fn primitives() -> usize;
    fn data_type() -> GLenum;

    /// Generates the type that would be used to represent this component in a
    /// glsl shader
    fn get_glsl_type() -> String {
        let primitives = <Self as VertexComponent>::primitives();
        let data_type = <Self as VertexComponent>::data_type();

        let mut result = String::with_capacity(4);

        if primitives == 1 {
            match data_type {
                gl::FLOAT => result.push_str("float"),
                gl::INT => result.push_str("int"),
                gl::UNSIGNED_INT => result.push_str("uint"),
                _ => ()
            }
        } else if primitives > 1 && primitives <= 4 {
            match data_type {
                gl::FLOAT => result.push_str("vec"),
                gl::INT => result.push_str("ivec"),
                gl::UNSIGNED_INT => result.push_str("uvec"),
                _ => ()
            }
            result.push_str(&primitives.to_string());
        }

        if result.is_empty() {
            panic!("Invalid VertexComponent: {} primitives of type {}", primitives, data_type);
        }

        result
    }
}

// Implementations for primitives and tuples (Maybe use generics here)
impl VertexComponent for f32 {
    fn bytes() -> usize { std::mem::size_of::<f32>() * 1 }
    fn primitives() -> usize { 1 }
    fn data_type() -> GLenum { gl::FLOAT }
}
impl VertexComponent for (f32, f32) {
    fn bytes() -> usize { std::mem::size_of::<f32>() * 2 }
    fn primitives() -> usize { 2 }
    fn data_type() -> GLenum { gl::FLOAT }
}
impl VertexComponent for (f32, f32, f32) {
    fn bytes() -> usize { std::mem::size_of::<f32>() * 3 }
    fn primitives() -> usize { 3 }
    fn data_type() -> GLenum { gl::FLOAT }
}
impl VertexComponent for (f32, f32, f32, f32) {
    fn bytes() -> usize { std::mem::size_of::<f32>() * 4 }
    fn primitives() -> usize { 4 }
    fn data_type() -> GLenum { gl::FLOAT }
}

impl VertexComponent for u32 {
    fn bytes() -> usize { std::mem::size_of::<u32>() * 1 }
    fn primitives() -> usize { 1 }
    fn data_type() -> GLenum { gl::UNSIGNED_INT }
}
impl VertexComponent for (u32, u32) {
    fn bytes() -> usize { std::mem::size_of::<u32>() * 2 }
    fn primitives() -> usize { 2 }
    fn data_type() -> GLenum { gl::UNSIGNED_INT }
}
impl VertexComponent for (u32, u32, u32) {
    fn bytes() -> usize { std::mem::size_of::<u32>() * 3 }
    fn primitives() -> usize { 3 }
    fn data_type() -> GLenum { gl::UNSIGNED_INT }
}
impl VertexComponent for (u32, u32, u32, u32) {
    fn bytes() -> usize { std::mem::size_of::<u32>() * 4 }
    fn primitives() -> usize { 4 }
    fn data_type() -> GLenum { gl::UNSIGNED_INT }
}

impl VertexComponent for i32 {
    fn bytes() -> usize { std::mem::size_of::<i32>() * 1 }
    fn primitives() -> usize { 1 }
    fn data_type() -> GLenum { gl::INT }
}
impl VertexComponent for (i32, i32) {
    fn bytes() -> usize { std::mem::size_of::<i32>() * 2 }
    fn primitives() -> usize { 2 }
    fn data_type() -> GLenum { gl::INT }
}
impl VertexComponent for (i32, i32, i32) {
    fn bytes() -> usize { std::mem::size_of::<i32>() * 3 }
    fn primitives() -> usize { 3 }
    fn data_type() -> GLenum { gl::INT }
}
impl VertexComponent for (i32, i32, i32, i32) {
    fn bytes() -> usize { std::mem::size_of::<i32>() * 4 }
    fn primitives() -> usize { 4 }
    fn data_type() -> GLenum { gl::INT }
}


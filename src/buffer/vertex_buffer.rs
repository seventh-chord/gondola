
use super::*;
use gl;
use gl::types::*;
use std;
use std::ops::Deref;

/// A GPU buffer which holds a list of a custom vertex type. This struct also has utility methods
/// for rendering the vertices as primitives.
///
/// # Deriving [`Vertex`](trait.Vertex.html)
/// A custom is defined in `gondola_derive` that can be used to derive the
/// [`Vertex`](trait.Vertex.html) trait for custom structs. For this to work, all members of
/// the struct need to implement [`VertexData`](trait.VertexData.html). See the
/// trait documentation for a list of implementations.
///
/// # Example - Rendering with a custom shader and vertex type
///
/// ```rust,no_run
/// #[macro_use] // Needed for load_shader! macro
/// extern crate gondola;
/// #[macro_use] // Provides custom derive for Vertex
/// extern crate gondola_derive; 
///
/// use gondola::buffer::{VertexBuffer, PrimitiveMode};
/// use gondola::shader::{Shader, ShaderPrototype};
///
/// #[repr(C)]
/// #[derive(Vertex)]
/// struct Vertex {
///     pos: (f32, f32),
/// }
///
/// # fn main() {
/// let data = vec![
///     Vertex { pos: (0.0, 0.0) },
///     Vertex { pos: (100.0, 0.0) },
///     Vertex { pos: (0.0, 100.0) },
/// ];
/// let buffer = VertexBuffer::with_data(PrimitiveMode::Triangles, &data);
///
/// // Creates a shader with input declarations for the custom type inserted
/// let shader = load_shader!("assets/shader.glsl", Vertex).unwrap();
///
/// shader.bind();
/// buffer.draw();
/// # }
/// ```
pub struct VertexBuffer<T: Vertex> {
    // We are generic over the vertex type, but dont actually store any vertices
    phantom: std::marker::PhantomData<T>,

    vertex_count: usize,
    allocated: usize,

    primitive_mode: PrimitiveMode,
    usage: BufferUsage,

    vbo: GLuint,
    vao: GLuint,
}

/// A GPU buffer which, similarly to [`VertexBuffer`], holds a list of a custom vertex type. Differently
/// from [`VertexBuffer`] a `IndexedVertexBuffer` uses a element/index buffer to render the
/// vertices in a non-default order. This is commonly used when rendering models or other complex
/// geometry.
/// 
/// This type has two generics parameters. `T` specifies the type of vertices which is
/// stored in this buffer, while `E` specifies the type of indices used. `E` must have a primitive
/// type which can be used as a index. All basic unsigned integers can be used as indices. For
/// further information, see [`GlIndex`].
///
/// Note that you can use a custom struct as a index. This allows you to statically enforce that
/// there is allways the correct number of indices to draw a primitive.
///
/// This struct dereferences to [`VertexBuffer`], exposing methods to modify the data in this
/// buffer. See its documentation for more information.
///
/// # Example - Using custom index types
/// ```rust,no_run
/// extern crate gondola;
/// #[macro_use]
/// extern crate gondola_derive; // Provides custom derive for Vertex
///
/// use gondola::buffer::{IndexedVertexBuffer, VertexData, PrimitiveMode};
///
/// #[repr(C)]
/// #[derive(Vertex)]
/// struct Vertex {
///     pos: (f32, f32),
/// }
///
/// #[repr(C)]
/// struct Triangle(u32, u32, u32);
///
/// // u32 can be used as a index, so Triangle can now also be used as a set of 3 indices
/// impl VertexData for Triangle {
///     type Primitive = u32; 
/// }
///
/// # fn main() {
/// let data = [
///     Vertex { pos: (0.0, 0.0) },
///     Vertex { pos: (10.0, 0.0) },
///     Vertex { pos: (10.0, 10.0) },
///     Vertex { pos: (0.0, 10.0) },
/// ];
/// let indices = [
///     Triangle(0, 1, 2),
///     Triangle(0, 2, 3),
/// ];
///
/// let buffer = IndexedVertexBuffer::with_data(PrimitiveMode::Triangles, &data, &indices);
/// # }
/// ```
///
/// [`VertexBuffer`]:        struct.VertexBuffer.html
/// [`GlIndex`]:             trait.GlIndex.html
pub struct IndexedVertexBuffer<T: Vertex, E: VertexData> where E::Primitive: GlIndex {
    data: VertexBuffer<T>,
    indices: PrimitiveBuffer<E>,
}

impl<T: Vertex> VertexBuffer<T> {
    /// Creates a new vertex buffer, preallocating space for 100 vertices.
    pub fn new(primitive_mode: PrimitiveMode, usage: BufferUsage) -> VertexBuffer<T> {
        VertexBuffer::with_capacity(primitive_mode, usage, DEFAULT_SIZE)
    }

    /// Creates a new vertex buffer, preallocating space for the given number of vertices.
    pub fn with_capacity(primitive_mode: PrimitiveMode, usage: BufferUsage, initial_capacity: usize) -> VertexBuffer<T> {
        let bytes = T::bytes_per_vertex() * initial_capacity;

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
            allocated: initial_capacity,

            primitive_mode: primitive_mode,
            usage: usage,

            vbo: vbo,
            vao: vao,
        }
    }

    /// Creates a new vertex buffer, storing the given vertices on the GPU.
    pub fn with_data(primitive_mode: PrimitiveMode, data: &[T]) -> VertexBuffer<T> {
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
    /// already in it. This resizes the underlying buffer if more space is needed
    /// to store the new data.
    pub fn put_at_end(&mut self, data: &[T]) {
        let vertex_count = self.vertex_count;
        self.put(vertex_count, data);
    }
    /// Puts the given vertices at the given index in this buffer, overwriting any
    /// vertices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new data.
    pub fn put(&mut self, index: usize, data: &[T]) {
        if data.is_empty() {
            return;
        }

        let start = index;
        let end = index + data.len();

        if end > self.allocated {
            self.ensure_allocated(end); // This currently does not allocate extra space
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

    /// Empties this buffer, setting its length to 0. This does nothing to the data
    /// stored in the buffer, it simply marks all current data as invalid.
    pub fn clear(&mut self) {
        self.vertex_count = 0;
    }

    /// The number of vertices that are stored in GPU memory.
    pub fn len(&self) -> usize {
        self.vertex_count
    }

    /// The number of vertices that can be stored in this buffer without
    /// reallocating memory. 
    pub fn capacity(&self) -> usize {
        self.allocated
    }

    /// Sets the number of vertices that can be stored in this buffer without
    /// reallocating memory. If the buffer already has capacity for the given
    /// number of vertices no space will be allocated.
    pub fn ensure_allocated(&mut self, new_size: usize) {
        // Only reallocate if necessary
        if new_size > self.allocated {
            let mut new_buffer = 0;
            let bytes = new_size * T::bytes_per_vertex();

            unsafe {
                gl::GenBuffers(1, &mut new_buffer);
                gl::BindBuffer(BufferTarget::Array as GLenum, new_buffer);
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

            self.vbo = new_buffer;
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

impl<T: Vertex, E: VertexData> IndexedVertexBuffer<T, E> 
    where E::Primitive: GlIndex,
{
    /// Creates a new indexed vertex buffer, preallocating space for 100 vertices and 100 indices.
    pub fn new(primitive_mode: PrimitiveMode, usage: BufferUsage) -> IndexedVertexBuffer<T, E> {
        IndexedVertexBuffer::with_capacity(primitive_mode, usage, DEFAULT_SIZE, DEFAULT_SIZE)
    }

    /// Creates a new indexed vertex buffer, preallocating space for the given number of vertices
    /// and indices.
    pub fn with_capacity(primitive_mode: PrimitiveMode, usage: BufferUsage, 
                         vertex_capacity: usize, index_capacity: usize) -> IndexedVertexBuffer<T, E> {
        let data = VertexBuffer::with_capacity(primitive_mode, usage, vertex_capacity);
        let indices = PrimitiveBuffer::with_capacity(BufferTarget::ElementArray, usage, index_capacity);

        IndexedVertexBuffer {
            data: data,
            indices: indices,
        }
    }

    /// Creates a new vertex buffer, storing the given vertices and indices on the GPU.
    pub fn with_data(primitive_mode: PrimitiveMode, data: &[T], indices: &[E]) -> IndexedVertexBuffer<T, E> {
        let data = VertexBuffer::with_data(primitive_mode, data);
        let indices = PrimitiveBuffer::with_data(BufferTarget::ElementArray, indices);

        IndexedVertexBuffer {
            data: data,
            indices: indices,
        }
    }

    /// Puts the given indices at the start of this buffer, replacing any indices
    /// which where previously in that location. This resizes the underlying buffer
    /// if more space is needed to store the new data.
    pub fn put_indices_at_start(&mut self, data: &[E]) {
        self.indices.put_at_start(data);
    }
    /// Puts the given indices at the end of this buffer, behind any data which is
    /// already in it. This resizes the underlying buffer if more space is needed
    /// to store the new data.
    pub fn put_indices_at_end(&mut self, data: &[E]) {
        self.indices.put_at_end(data);
    }
    /// Puts the given indices at the given index in this buffer, overwriting any
    /// indices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new data.
    pub fn put(&mut self, index: usize, data: &[E]) {
        self.indices.put(index, data);
    }

    /// Empties this buffers index buffer, setting its length to 0. This does nothing to the data
    /// stored in the buffer, it simply marks all current data as invalid.
    pub fn clear_indices(&mut self) {
        self.indices.clear();
    }

    /// The number of indices that are stored in GPU memory.
    pub fn index_len(&self) -> usize {
        self.indices.len()
    }

    /// The number of indices that can be stored in this buffer without
    /// reallocating memory. 
    pub fn index_capacity(&self) -> usize {
        self.allocated
    }

    /// Sets the number of indices that can be stored in this buffer without
    /// reallocating memory. If the buffer already has capacity for the given
    /// number of indices no space will be allocated.
    pub fn ensure_indices_allocated(&mut self, new_size: usize) {
        self.indices.ensure_allocated(new_size);
    }

    /// Draws the contents of this vertex buffer with the primitive mode specified
    /// at construction and the index/element buffer.
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.data.vao);
            gl::DrawElements(self.data.primitive_mode as GLenum, 
                             (self.indices.len() * E::primitives()) as GLsizei,
                             E::Primitive::gl_enum(), std::ptr::null());
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

impl<T: Vertex, E: VertexData> Deref for IndexedVertexBuffer<T, E> 
    where E::Primitive: GlIndex,
{
    type Target = VertexBuffer<T>;
    fn deref(&self) -> &VertexBuffer<T> {
        &self.data
    }
}


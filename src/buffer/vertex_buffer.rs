
use std;
use std::ops::Range;

use gl;
use gl::types::*;

use super::*;

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

    vertex_count: usize, // Used space, in number of vertices
    allocated: usize, // Allocated space, in number of vertices

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
    vertices: VertexBuffer<T>,
    indices: PrimitiveBuffer<E>,
}

impl<T: Vertex> VertexBuffer<T> {
    /// Creates a new vertex buffer without allocating
    pub fn new(primitive_mode: PrimitiveMode, usage: BufferUsage) -> VertexBuffer<T> {
        let vbo = 0; // Not set yet
        let mut vao = 0;

        unsafe { gl::GenVertexArrays(1, &mut vao) };

        VertexBuffer {
            phantom: std::marker::PhantomData,
            vertex_count: 0,
            allocated: 0,

            primitive_mode, usage,
            vbo, vao,
        }
    }

    /// Creates a new vertex buffer, preallocating space for the given number of vertices.
    pub fn with_capacity(primitive_mode: PrimitiveMode, usage: BufferUsage, initial_capacity: usize) -> VertexBuffer<T> {
        let mut buffer = VertexBuffer::new(primitive_mode, usage);
        let bytes = T::bytes_per_vertex() * initial_capacity;

        unsafe {
            gl::GenBuffers(1, &mut buffer.vbo);
            gl::BindBuffer(BufferTarget::Array as GLenum, buffer.vbo);
            gl::BufferData(BufferTarget::Array as GLenum, bytes as GLsizeiptr, std::ptr::null(), usage as GLenum);

            gl::BindVertexArray(buffer.vao);
            T::setup_attrib_pointers();
        }

        buffer.vertex_count = 0;
        buffer.allocated = initial_capacity;

        return buffer;
    }

    /// Creates a new vertex buffer, storing the given vertices on the GPU.
    pub fn with_data(primitive_mode: PrimitiveMode, vertices: &[T]) -> VertexBuffer<T> {
        let usage = BufferUsage::StaticDraw;
        let mut buffer = VertexBuffer::new(primitive_mode, usage);

        let vertex_count = vertices.len();
        let bytes = T::bytes_per_vertex() * vertex_count;

        unsafe {
            gl::GenBuffers(1, &mut buffer.vbo);
            gl::BindBuffer(BufferTarget::Array as GLenum, buffer.vbo);
            gl::BufferData(
                BufferTarget::Array as GLenum,
                bytes as GLsizeiptr,
                std::mem::transmute(&vertices[0]),
                usage as GLenum
            );

            gl::BindVertexArray(buffer.vao);
            T::setup_attrib_pointers();
        }

        buffer.vertex_count = vertex_count;
        buffer.allocated    = vertex_count;

        return buffer;
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

        let full_override = start == 0 && end >= self.vertex_count;
        self.ensure_allocated(end, !full_override);

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

    /// Ensures that the capacity of this buffer is `new_capacity`. If necessary, this reallocates
    /// the internal buffer. If the internal buffer is allready big enough this function does
    /// nothing. `new_capacity` is in units of `T`.
    /// If `retain_old_data` is `false` this will zero out all data if it decides to reallocate
    pub fn ensure_allocated(&mut self, new_capacity: usize, retain_old_data: bool) {
        if new_capacity > self.allocated {
            let mut new_buffer = 0;
            let bytes = new_capacity * T::bytes_per_vertex();

            unsafe {
                gl::GenBuffers(1, &mut new_buffer);
                gl::BindBuffer(BufferTarget::Array as GLenum, new_buffer);
                gl::BufferData(BufferTarget::Array as GLenum, bytes as GLsizeiptr, std::ptr::null(), self.usage as GLenum);

                gl::BindVertexArray(self.vao);
                T::setup_attrib_pointers();

                // Copy old data
                if retain_old_data && self.vbo != 0 {
                    gl::BindBuffer(BufferTarget::CopyRead as GLenum, self.vbo);
                    gl::CopyBufferSubData(
                        BufferTarget::CopyRead as GLenum,
                        BufferTarget::Array as GLenum,
                        0, 0,
                        (self.vertex_count * T::bytes_per_vertex()) as GLsizeiptr
                    );
                    gl::DeleteBuffers(1, &mut self.vbo);
                }
            }

            self.vbo = new_buffer;
            self.allocated = new_capacity
        }
    }

    /// Draws the contents of this vertex buffer with the primitive mode specified at construction.
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(self.primitive_mode as GLenum, 0, self.vertex_count as GLsizei);
        }
    }

    /// Draws a subrange of the contents of this vertex buffer with the primitive mode specified at
    /// construction. The start of the range is inclusive, and the end of the range is exclusive.
    /// Panics if the range is outside of the bounds of this buffer, or the start of
    /// the range lies after the end of the range.
    ///
    /// The range is in units of the vertex type `T` used with this buffer. For example, in a buffer
    /// with 6 vertices constituting 2 triangles, `draw_range(3..6)` will draw the second triangle.
    pub fn draw_range(&self, range: Range<usize>) {
        assert!(range.start < range.end, 
                "Call to draw_range with invalid range {}..{}, start must lie before end!",
                range.start, range.end);
        assert!(range.end <= self.vertex_count,
                "Call to draw_range with invalid range {}..{}, end or range lies beyond end \
                of buffer (len = {})", range.start, range.end, self.vertex_count);

        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(self.primitive_mode as GLenum, range.start as GLint, (range.end - range.start) as GLsizei);
        }
    }

    /// Draws the contents of this vertex buffer, feeding transform feedback data into the given
    /// buffer. If `rasterization` is set to false the fragment shader will not be run and no data
    /// will be written to the bound framebuffer.
    pub fn transform_feedback_into<U>(&self, target: &mut VertexBuffer<U>, rasterization: bool) 
      where U: Vertex,
    {
        unsafe {
            if !rasterization { gl::Enable(gl::RASTERIZER_DISCARD); }

            gl::BindVertexArray(self.vao);
            gl::BindBufferBase(gl::TRANSFORM_FEEDBACK_BUFFER, 0, target.vbo);
            gl::BeginTransformFeedback(self.primitive_mode.gl_base_primitive() as GLenum);
            gl::DrawArrays(self.primitive_mode as GLenum, 0, self.vertex_count as GLsizei);
            gl::EndTransformFeedback();

            if !rasterization { gl::Disable(gl::RASTERIZER_DISCARD); }
        }
    }
}

impl<T: Vertex, E: VertexData> IndexedVertexBuffer<T, E> 
  where E::Primitive: GlIndex,
{
    /// Creates a new indexed vertex buffer, preallocating space for 100 vertices and 100 indices.
    pub fn new(primitive_mode: PrimitiveMode, usage: BufferUsage) -> IndexedVertexBuffer<T, E> {
        IndexedVertexBuffer {
            vertices: VertexBuffer::new(primitive_mode, usage),
            indices:  PrimitiveBuffer::new(BufferTarget::ElementArray, usage),
        }
    }

    /// Creates a new indexed vertex buffer, preallocating space for the given number of vertices
    /// and indices.
    pub fn with_capacity(primitive_mode: PrimitiveMode, usage: BufferUsage, 
                         vertex_capacity: usize, index_capacity: usize) -> IndexedVertexBuffer<T, E> {
        IndexedVertexBuffer {
            vertices: VertexBuffer::with_capacity(primitive_mode, usage, vertex_capacity), 
            indices:  PrimitiveBuffer::with_capacity(BufferTarget::ElementArray, usage, index_capacity),
        }
    }

    /// Creates a new vertex buffer, storing the given vertices and indices on the GPU.
    pub fn with_data(primitive_mode: PrimitiveMode, vertices: &[T], indices: &[E]) -> IndexedVertexBuffer<T, E> {
        IndexedVertexBuffer {
            vertices: VertexBuffer::with_data(primitive_mode, vertices),
            indices:  PrimitiveBuffer::with_data(BufferTarget::ElementArray, indices),
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
    pub fn put_indices(&mut self, index: usize, data: &[E]) {
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
        self.indices.capacity()
    }

    /// Sets the number of indices that can be stored in this buffer without
    /// reallocating memory. If the buffer already has capacity for the given
    /// number of indices no space will be allocated.
    pub fn ensure_indices_allocated(&mut self, new_size: usize, retain_old_data: bool) {
        self.indices.ensure_allocated(new_size, retain_old_data);
    }


    /// Puts the given vertices at the start of this buffer, replacing any vertices
    /// which where previously in that location. This resizes the underlying buffer
    /// if more space is needed to store the new data.
    pub fn put_vertices_at_start(&mut self, data: &[T]) {
        self.vertices.put_at_start(data);
    }

    /// Puts the given vertices at the end of this buffer, behind any vertices which are already in
    /// it. This resizes the underlying buffer if more space is needed to store the new vertices.
    /// If `retain_old_data` is `false` this will zero out all indices if it decides to reallocate
    pub fn put_vertices_at_end(&mut self, data: &[T]) {
        self.vertices.put_at_end(data);
    }

    /// Puts the given vertices at the given vertex in this buffer, overwriting any
    /// vertices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new vertices.
    pub fn put_vertices(&mut self, vertex: usize, data: &[T]) {
        self.vertices.put(vertex, data);
    }

    /// Empties this buffers vertex buffer, setting its length to 0. This does nothing to the data
    /// stored in the buffer, it simply marks all current data as invalid.
    pub fn clear_vertices(&mut self) {
        self.vertices.clear();
    }

    /// The number of vertices that are stored in GPU memory.
    pub fn vertex_len(&self) -> usize {
        self.vertices.len()
    }

    /// The number of vertices that can be stored in this buffer without
    /// reallocating memory.
    pub fn vertex_capacity(&self) -> usize {
        self.vertices.capacity()
    }

    /// Sets the number of vertices that can be stored in this buffer without reallocating memory.
    /// If the buffer already has capacity for the given number of vertices no space will be
    /// allocated.
    /// If `retain_old_data` is `false` this will zero out all data if it decides to reallocate
    pub fn ensure_vertices_allocated(&mut self, new_size: usize, retain_old_data: bool) {
        self.vertices.ensure_allocated(new_size, retain_old_data);
    }


    /// Draws the contents of this vertex buffer with the primitive mode specified
    /// at construction and the index/element buffer.
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vertices.vao);
            gl::DrawElements(
                self.vertices.primitive_mode as GLenum,
                (self.indices.len() * E::primitives()) as GLsizei,
                E::Primitive::gl_enum(),
                std::ptr::null(),
            );
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

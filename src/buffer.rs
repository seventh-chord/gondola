
//! Utilities for storing and drawing data in GPU buffers

use gl;
use gl::types::*;
use std;
use std::ops::Range;
use cable_math::{Vec2, Vec3, Vec4};

const DEFAULT_SIZE: usize = 100;

/// A GPU buffer which holds a list of a custom vertex type. This struct also has utility methods
/// for rendering the vertices as primitives.
///
/// # Deriving [`Vertex`](trait.Vertex.html)
/// A custom proc_macro is defined in `gondola_vertex_macro` that can be used to derive the
/// [`Vertex`](trait.Vertex.html) trait for custom structs. For this to work, all members of
/// the struct need to implement [`VertexComponent`](trait.VertexComponent.html). See the
/// trait documentation for a list of implementations.
///
/// # Example - Rendering with a custom shader and vertex type
///
/// Imports:
///
/// ```
/// extern crate cable_math;
/// #[macro_use]
/// extern crate gondola_derive; // Provides custom derive for Vertex
///
/// use cable_math::Vec2;
/// use gondola::buffer::VertexBuffer;
/// use gondola::shader::{Shader, ShaderPrototype}
/// ```
///
/// Vertex declaration:
///
/// ```
/// #[derive(Vertex)]
/// struct Point {
///     position: Vec2<f32>,
/// }
/// ```
///
/// Usage:
///
/// ```
/// let data = vec![
///     Point { position: Vec2::new(0.0, 0.0) },
///     Point { position: Vec2::new(100.0, 0.0) },
///     Point { position: Vec2::new(0.0, 100.0) },
/// ];
/// let buffer = VertexBuffer::from_data(PrimitiveMode::Triangles, &data);
///
/// // Creates a shader with input declarations for the custom type inserted
/// let shader = load_shader!("assets/shader.glsl", Point).unwrap();
///
/// shader.bind();
/// buffer.draw();
/// ```
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
    /// realocating memory. If the buffer allready has capacity for the given
    /// number of vertices no space will be allocated.
    pub fn ensure_allocated(&mut self, new_size: usize) {
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

/// A GPU buffer which holds a set of primitives (floats, bytes or integers). These primitives
/// can be rendered using a [`VertexArray`](struct.VertexArray.html).
pub struct PrimitiveBuffer {
    buffer: GLuint,
    target: BufferTarget,
    usage: BufferUsage,
    allocated: usize,
    primitives: usize,
    data_type: DataType,
}

impl PrimitiveBuffer {
    /// Initializes a new, empty, buffer
    pub fn new(target: BufferTarget, usage: BufferUsage, data_type: DataType) -> PrimitiveBuffer {
        let mut buffer = 0;

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(target as GLenum, DEFAULT_SIZE as GLsizeiptr, std::ptr::null(), usage as GLenum);
        }

        PrimitiveBuffer {
            buffer: buffer,
            target: target,
            usage: usage,
            allocated: DEFAULT_SIZE,
            primitives: 0,
            data_type: data_type,
        }
    }

    /// Stores the given data in a new buffer. The buffer will have its usage set to `BufferUsage::StaticDraw`
    pub fn from_floats(target: BufferTarget, data: &[f32]) -> PrimitiveBuffer {
        let mut buffer = 0;
        let byte_count = data.len() * DataType::Float.size(); // We assume f32 to be equal to GLfloat, which it is

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(
                target as GLenum,
                byte_count as GLsizeiptr,
                std::mem::transmute(&data[0]),
                BufferUsage::StaticDraw as GLenum
            );
        }

        PrimitiveBuffer {
            buffer: buffer,
            target: target,
            usage: BufferUsage::StaticDraw,
            allocated: byte_count,
            primitives: data.len(),
            data_type: DataType::Float
        }
    }

    /// Stores the given vector into this buffer, overwriting any data that was 
    /// previously in the buffer
    pub fn put_floats(&mut self, data: &[f32]) {
        self.data_type = DataType::Float;
        self.primitives = data.len();
        let byte_count = data.len() * DataType::Float.size(); 

        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);

            //Resize if necesarry
            if self.allocated < byte_count {
                gl::BufferData(
                    self.target as GLenum,
                    byte_count as GLsizeiptr,
                    std::mem::transmute(&data[0]),
                    self.usage as GLenum
                );
            } else {
                gl::BufferSubData(
                    self.target as GLenum,
                    0 as GLintptr, byte_count as GLsizeiptr,
                    std::mem::transmute(&data[0])
                );
            }
        }
    }

    /// The number of primitives that are stored in GPU memory. Note that this is
    /// *different* from the number of bytes stored.
    pub fn len(&self) -> usize {
        self.primitives
    }

    /// The number of bytes that are internally allocated in GPU memory
    pub fn bytes_allocated(&self) -> usize {
        self.allocated
    }

    /// The type of data that is stored in the buffer
    pub fn data_type(&self) -> DataType {
        self.data_type
    }

    /// Binds this buffer to the target specified in the constructor
    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);
        }
    }
}

impl Drop for PrimitiveBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.buffer);
        }
    }
}

/// Contains information on how to render a group of primitive buffers
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

    /// Adds a buffer from which this vertex array will pull data when drawing
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

/// Represents different types of primitives which can be drawn on the GPU.
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

/// Represents different GL buffer usage hints. Note that these are hints,
/// and drivers will not necesarily respect these.
///
/// The first part of the name indicates how frequently the data will be used,
/// the specond part indicates how it will be used:
///
/// * Static - Data is set once and used often 
/// * Dynamic - Data is set frequently and used frequently
/// * Stream - Data is set once and used at most a few times
///
/// * Draw - Data will be set by the application and read by the GPU
/// * Read - Data is set by the GPU and read by the application
/// * Copy - Data is set and read by the GPU
#[derive(Copy, Clone)]
pub enum BufferUsage {
    StaticDraw  = gl::STATIC_DRAW as isize,
    DynamicDraw = gl::DYNAMIC_DRAW as isize,
    StreamDraw  = gl::STREAM_DRAW as isize,
    StaticRead  = gl::STATIC_READ as isize,
    DynamicRead = gl::DYNAMIC_READ as isize,
    StreamRead  = gl::STREAM_READ as isize,
    StaticCopy  = gl::STATIC_COPY as isize,
    DynamicCopy = gl::DYNAMIC_COPY as isize,
    StreamCopy  = gl::STREAM_COPY as isize,
}

/// Reperesents a target to which a buffer can be bound
#[derive(Copy, Clone)]
pub enum BufferTarget {
    Array               = gl::ARRAY_BUFFER as isize,
    ElementArray        = gl::ELEMENT_ARRAY_BUFFER as isize,
    PixelPack           = gl::PIXEL_PACK_BUFFER as isize,
    PixelUnpack         = gl::PIXEL_UNPACK_BUFFER as isize,
    TransformFeedback   = gl::TRANSFORM_FEEDBACK_BUFFER as isize,
    Uniform             = gl::UNIFORM_BUFFER as isize,
    Texture             = gl::TEXTURE_BUFFER as isize,
    CopyRead            = gl::COPY_READ_BUFFER as isize,
    CopyWrite           = gl::COPY_WRITE_BUFFER as isize,
    DrawIndirect        = gl::DRAW_INDIRECT_BUFFER as isize,
    AtomicCounter       = gl::ATOMIC_COUNTER_BUFFER as isize,
    DispatchIndirect    = gl::DISPATCH_INDIRECT_BUFFER as isize,
}

/// Represents different types of data which may be stored in a buffer
#[derive(Copy, Clone)]
pub enum DataType {
    Float        = gl::FLOAT as isize,
    Int          = gl::INT as isize,
    Byte         = gl::BYTE as isize,
    UnsignedInt  = gl::UNSIGNED_INT as isize,
    UnsignedByte = gl::UNSIGNED_BYTE as isize,
}
impl DataType {
    pub fn size(&self) -> usize {
        match *self {
            DataType::Float        => std::mem::size_of::<f32>(),
            DataType::Int          => std::mem::size_of::<i32>(),
            DataType::Byte         => std::mem::size_of::<i8>(),
            DataType::UnsignedInt  => std::mem::size_of::<u32>(),
            DataType::UnsignedByte => std::mem::size_of::<u8>(),
        }
    }
}

/// Vertex buffers store a list of `Vertex`es (called vertices in proper
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
// Implementations for primitives and tuples
macro_rules! impl_vertex_component {
    ($primitive:ty, $data_type:expr) => {
        // Implement for single values, and 2-4 tuples
        impl VertexComponent for $primitive {
            fn bytes() -> usize { std::mem::size_of::<$primitive>() }
            fn primitives() -> usize { 1 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
        impl VertexComponent for ($primitive, $primitive) {
            fn bytes() -> usize { std::mem::size_of::<$primitive>() * 2 }
            fn primitives() -> usize { 2 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
        impl VertexComponent for ($primitive, $primitive, $primitive) {
            fn bytes() -> usize { std::mem::size_of::<$primitive>() * 3 }
            fn primitives() -> usize { 3 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
        impl VertexComponent for ($primitive, $primitive, $primitive, $primitive) {
            fn bytes() -> usize { std::mem::size_of::<$primitive>() * 4 }
            fn primitives() -> usize { 4 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
        impl VertexComponent for Vec2<$primitive> {
            fn bytes() -> usize { std::mem::size_of::<Vec2<$primitive>>() }
            fn primitives() -> usize { 2 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
        impl VertexComponent for Vec3<$primitive> {
            fn bytes() -> usize { std::mem::size_of::<Vec3<$primitive>>() }
            fn primitives() -> usize { 3 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
        impl VertexComponent for Vec4<$primitive> {
            fn bytes() -> usize { std::mem::size_of::<Vec4<$primitive>>() }
            fn primitives() -> usize { 4 }
            fn data_type() -> GLenum { $data_type as GLenum }
        }
    }
}
impl_vertex_component!(f32, DataType::Float);
impl_vertex_component!(i32, DataType::Int);
impl_vertex_component!(u32, DataType::UnsignedInt);
impl_vertex_component!(i8, DataType::Byte);
impl_vertex_component!(u8, DataType::UnsignedByte);


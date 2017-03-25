
//! Utilities for storing and drawing data in GPU buffers

use gl;
use gl::types::*;
use std;
use std::mem::size_of;
use std::ops::Range;
use cable_math::{Vec2, Vec3, Vec4, Mat4};

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
/// ```rust,ignore
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
/// ```rust,ignore
/// #[derive(Vertex)]
/// struct Point {
///     position: Vec2<f32>,
/// }
/// ```
///
/// Usage:
///
/// ```rust,ignore
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

impl<T: Vertex> VertexBuffer<T> {
    /// Creates a new vertex buffer, prealocating space for 100 vertices.
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
pub struct PrimitiveBuffer<T: VertexComponent> {
    phantom: std::marker::PhantomData<T>,

    buffer: GLuint,
    target: BufferTarget,
    usage: BufferUsage,
    allocated: usize,
    used: usize,
}

impl<T: VertexComponent> PrimitiveBuffer<T> {
    /// Initializes a new, empty, buffer
    pub fn new(target: BufferTarget, usage: BufferUsage) -> PrimitiveBuffer<T> {
        let mut buffer = 0;

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(target as GLenum, (DEFAULT_SIZE * T::bytes()) as GLsizeiptr, std::ptr::null(), usage as GLenum);
        }

        PrimitiveBuffer {
            phantom: std::marker::PhantomData,

            buffer: buffer,
            target: target,
            usage: usage,
            allocated: DEFAULT_SIZE,
            used: 0,
        }
    }

    /// Stores the given data in a new buffer. The buffer will have its usage set to `BufferUsage::StaticDraw`
    pub fn with_data(target: BufferTarget, data: &[T]) -> PrimitiveBuffer<T> {
        if data.is_empty() {
            return PrimitiveBuffer::new(target, BufferUsage::StaticDraw);
        }

        let mut buffer = 0;
        let byte_count = data.len() * T::bytes();

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(target as GLenum,
                           byte_count as GLsizeiptr,
                           std::mem::transmute(&data[0]),
                           BufferUsage::StaticDraw as GLenum);
        }

        PrimitiveBuffer {
            phantom: std::marker::PhantomData,

            buffer: buffer,
            target: target,
            usage: BufferUsage::StaticDraw,
            allocated: byte_count,
            used: data.len() * T::bytes(),
        }
    }
    
    /// Puts the given data at the start of this buffer, replacing any vertices
    /// which where previously in that location. This resizes the underlying buffer
    /// if more space is needed to store the new data.
    pub fn put_at_start(&mut self, data: &[T]) {
        self.put(0, data);
    }
    /// Puts the given data at the end of this buffer, behind any data which is
    /// allready in it. This resizes the underlying buffer if more space is needed
    /// to store the new data.
    pub fn put_at_end(&mut self, data: &[T]) {
        let end = self.used;
        self.put(end, data);
    }
    /// Puts the given data at the given index in this buffer, overwriting any
    /// vertices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new data.
    ///
    /// The index should be in units of the size of `T`. Thus, for a `PrimitiveBuffer<f32>`, a
    /// index of `2` will start writing data at the eight byte.
    pub fn put(&mut self, index: usize, data: &[T]) {
        if data.is_empty() {
            return;
        }

        let start = index*T::bytes();
        let end = index + data.len()*T::bytes();

        if end > self.allocated {
            self.ensure_allocated(end); // This currently does not allocate extra space
        }

        if end > self.used {
            self.used = end;
        }

        unsafe {
            gl::BindBuffer(BufferTarget::Array as GLenum, self.buffer);
            gl::BufferSubData(
                BufferTarget::Array as GLenum,
                (start * T::bytes()) as GLintptr,
                (data.len() * T::bytes()) as GLsizeiptr,
                std::mem::transmute(&data[0])
            );
        }
    }
    
    /// Sets the number of vertices that can be stored in this buffer without
    /// realocating memory. If the buffer allready has capacity for the given
    /// number of vertices no space will be allocated.
    pub fn ensure_allocated(&mut self, new_size: usize) {
        // Only realocate if necessary
        if new_size > self.allocated {
            let mut new_vbo = 0;

            unsafe {
                gl::GenBuffers(1, &mut new_vbo);
                gl::BindBuffer(BufferTarget::Array as GLenum, new_vbo);
                gl::BufferData(BufferTarget::Array as GLenum, new_size as GLsizeiptr, std::ptr::null(), self.usage as GLenum);

                // Copy old data
                gl::BindBuffer(BufferTarget::CopyRead as GLenum, self.buffer);
                gl::CopyBufferSubData(
                    BufferTarget::CopyRead as GLenum,
                    BufferTarget::Array as GLenum,
                    0, 0,
                    self.used as GLsizeiptr
                );
                gl::DeleteBuffers(1, &mut self.buffer);
            }

            self.buffer = new_vbo;
            self.allocated = new_size
        }
    }

    /// Empties this buffer by setting its length to 0.
    pub fn clear(&mut self) {
        self.used = 0;
    }

    /// The number of `T`s stored in this buffer
    pub fn len(&self) -> usize {
        self.used / T::bytes()
    }
    
    /// The number of primitives stored in this buffer. Note that a single `T` may contain
    /// multiple primitives.
    pub fn primitives(&self) -> usize {
        self.len() * T::primitives()
    }

    /// The number of bytes stored in this buffer
    pub fn bytes(&self) -> usize {
        self.used
    }

    /// The number of bytes that are internally allocated in GPU memory
    pub fn bytes_allocated(&self) -> usize {
        self.allocated
    }

    /// The type of data that is stored in the buffer
    pub fn data_type(&self) -> DataType {
        T::data_type()
    }

    /// Binds this buffer to the target specified in the constructor
    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);
        }
    }

    /// Calls `glBindBufferBase` for this buffer, with the given index. This is used
    /// in conjunctions with e.g. uniform buffers.
    pub fn bind_base(&mut self, index: GLuint) {
        unsafe {
            gl::BindBufferBase(self.target as GLenum, index, self.buffer);
        }
    }
}

impl<T: VertexComponent> Drop for PrimitiveBuffer<T> {
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
    ///
    /// # Parameters
    /// - `index`:  The vertex attribute index to which this data source will be bound. This is
    ///             used from glsl through `layout(location = index) in ...;`
    /// - `size`:   The number of primitives per vertex to use. e.g.: `3` means pull sets of three
    ///             vertices from the source and present them as a `vec3` in glsl.
    /// - `stride`: The distance from the start of the first vertex to the start of the next
    ///             vertex. e.g.: If you have a buffer with the contents 
    ///             `[x, y, z, r, g, b, x, y, z, r, g, b]`, you could use a stride of `6` to
    ///             indicate that you have to advance 6 primitives to get from one color to the
    ///             next color.
    /// - `offset`: The number of primitives at the beginning of the source to skip.
    pub fn add_data_source<T>(&self, source: &PrimitiveBuffer<T>, 
                              index: usize, size: usize, 
                              stride: usize, offset: usize) 
        where T: VertexComponent
    {
        source.bind();

        unsafe {
            gl::BindVertexArray(self.array);
            gl::EnableVertexAttribArray(index as GLuint);

            gl::VertexAttribPointer(index as GLuint, size as GLint,
                                    T::data_type() as GLenum, false as GLboolean,
                                    (stride * T::bytes()) as GLsizei, 
                                    (offset * T::bytes()) as *const GLvoid);
        }
    }

    /// Registers the given primitive buffer to be used as a element buffer for this vertex array.
    /// After this call, calls to `draw_elements` are safe.
    pub fn add_ebo<T>(&self, ebo: &PrimitiveBuffer<T>) 
        where T: VertexComponent
    {
        unsafe {
            gl::BindVertexArray(self.array);
            ebo.bind();
        } 
    }

    /// Draws the given primitives with the graphics buffers bound to this vertex array 
    pub fn draw(&self, mode: PrimitiveMode, range: Range<usize>) {
        unsafe {
            gl::BindVertexArray(self.array);
            gl::DrawArrays(mode as GLenum, range.start as GLint, (range.end - range.start) as GLsizei);
        }
    }

    /// Draws this vertex array using a previously set element buffer. `ebo_data_type` should be
    /// the type of data that is in the element buffer.
    pub fn draw_elements(&self, mode: PrimitiveMode, ebo_data_type: DataType, count: usize) {
        unsafe {
            gl::BindVertexArray(self.array);
            gl::DrawElements(mode as GLenum, count as GLsizei, ebo_data_type as GLenum, std::ptr::null());
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
#[repr(u32)] // GLenum is u32
#[derive(Copy, Clone)]
pub enum PrimitiveMode {
    Points                      = gl::POINTS,
    LineStrip                   = gl::LINE_STRIP,
    LineLoop                    = gl::LINE_LOOP,
    Lines                       = gl::LINES,
    LineStripAdjacency          = gl::LINE_STRIP_ADJACENCY,
    LinesAdjacency              = gl::LINES_ADJACENCY,
    TriangleStrip               = gl::TRIANGLE_STRIP,
    TriangleFan                 = gl::TRIANGLE_FAN,
    Triangles                   = gl::TRIANGLES,
    TriangleStripAdjacency      = gl::TRIANGLE_STRIP_ADJACENCY,
    TrianglesAdjacency          = gl::TRIANGLES_ADJACENCY,
} 

/// Represents different GL buffer usage hints. Note that these are hints,
/// and drivers will not necesarily respect these.
///
/// The first part of the name indicates how frequently the data will be used:  
///
/// * Static - Data is set once and used often 
/// * Dynamic - Data is set frequently and used frequently
/// * Stream - Data is set once and used at most a few times
///
/// The specond part indicates how it will be used:  
///
/// * Draw - Data will be set by the application and read by the GPU
/// * Read - Data is set by the GPU and read by the application
/// * Copy - Data is set and read by the GPU
#[repr(u32)] // GLenum is u32
#[derive(Copy, Clone)]
pub enum BufferUsage {
    StaticDraw  = gl::STATIC_DRAW,
    DynamicDraw = gl::DYNAMIC_DRAW,
    StreamDraw  = gl::STREAM_DRAW,
    StaticRead  = gl::STATIC_READ,
    DynamicRead = gl::DYNAMIC_READ,
    StreamRead  = gl::STREAM_READ,
    StaticCopy  = gl::STATIC_COPY,
    DynamicCopy = gl::DYNAMIC_COPY,
    StreamCopy  = gl::STREAM_COPY,
}

/// Reperesents a target to which a buffer can be bound
#[repr(u32)] // GLenum is u32
#[derive(Copy, Clone)]
pub enum BufferTarget {
    Array               = gl::ARRAY_BUFFER,
    ElementArray        = gl::ELEMENT_ARRAY_BUFFER,
    PixelPack           = gl::PIXEL_PACK_BUFFER,
    PixelUnpack         = gl::PIXEL_UNPACK_BUFFER,
    TransformFeedback   = gl::TRANSFORM_FEEDBACK_BUFFER,
    Uniform             = gl::UNIFORM_BUFFER,
    Texture             = gl::TEXTURE_BUFFER,
    CopyRead            = gl::COPY_READ_BUFFER,
    CopyWrite           = gl::COPY_WRITE_BUFFER,
    DrawIndirect        = gl::DRAW_INDIRECT_BUFFER,
    AtomicCounter       = gl::ATOMIC_COUNTER_BUFFER,
    DispatchIndirect    = gl::DISPATCH_INDIRECT_BUFFER,
}

/// Represents different types of data which may be stored in a buffer
#[repr(u32)] // GLenum is u32
#[derive(Debug, Copy, Clone)]
pub enum DataType {
    Float        = gl::FLOAT,
    Int          = gl::INT,
    Byte         = gl::BYTE,
    UnsignedInt  = gl::UNSIGNED_INT,
    UnsignedByte = gl::UNSIGNED_BYTE,
}
impl DataType {
    pub fn size(&self) -> usize {
        match *self {
            DataType::Float        => size_of::<f32>(),
            DataType::Int          => size_of::<i32>(),
            DataType::Byte         => size_of::<i8>(),
            DataType::UnsignedInt  => size_of::<u32>(),
            DataType::UnsignedByte => size_of::<u8>(),
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
    /// The total number of bytes one of these components takes.
    fn bytes() -> usize;
    /// The total number of primitives one of these components provides (e.g. 4 for a `Vec4<T>`).
    fn primitives() -> usize;
    /// The type of primitives this component provides.
    fn data_type() -> DataType;

    /// Generates the type that would be used to represent this component in a
    /// glsl shader
    fn get_glsl_type() -> String {
        let primitives = <Self as VertexComponent>::primitives();
        let data_type = <Self as VertexComponent>::data_type();

        let mut result = String::with_capacity(4);

        if primitives == 1 {
            match data_type {
                DataType::Float =>       result.push_str("float"),
                DataType::Int =>         result.push_str("int"),
                DataType::UnsignedInt => result.push_str("uint"),
                _ => panic!("Data type {:?} is not supported for glsl yet. See {}:{}", data_type, file!(), line!()),
            }
        } else if primitives > 1 && primitives <= 4 {
            match data_type {
                DataType::Float =>       result.push_str("vec"),
                DataType::Int =>         result.push_str("ivec"),
                DataType::UnsignedInt => result.push_str("uvec"),
                _ => panic!("Data type {:?} is not supported for glsl yet. See {}:{}", data_type, file!(), line!()),
            }
            result.push_str(&primitives.to_string());
        }

        if result.is_empty() {
            panic!("Invalid VertexComponent: {} primitives of type {:?}", primitives, data_type);
        }

        result
    }
}

// Implementations for VertexComponent
// Implementations for primitives
macro_rules! impl_vertex_component {
    ($primitive:ty, $data_type:expr) => {
        impl VertexComponent for $primitive {
            fn bytes() -> usize { size_of::<$primitive>() }
            fn primitives() -> usize { 1 }
            fn data_type() -> DataType { $data_type }
        }
    }
}
impl_vertex_component!(f32, DataType::Float);
impl_vertex_component!(i32, DataType::Int);
impl_vertex_component!(u32, DataType::UnsignedInt);
impl_vertex_component!(i8, DataType::Byte);
impl_vertex_component!(u8, DataType::UnsignedByte);

// Recursive generics woo!!!
impl<T: VertexComponent> VertexComponent for Mat4<T> {
    fn bytes() -> usize { size_of::<Mat4<T>>() }
    fn primitives() -> usize { 16 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for Vec2<T> {
    fn bytes() -> usize { size_of::<Vec2<T>>() }
    fn primitives() -> usize { 2 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for Vec3<T> {
    fn bytes() -> usize { size_of::<Vec3<T>>() }
    fn primitives() -> usize { 3 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for Vec4<T> {
    fn bytes() -> usize { size_of::<Vec4<T>>() }
    fn primitives() -> usize { 4 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for [T; 1] {
    fn bytes() -> usize { size_of::<[T; 1]>() }
    fn primitives() -> usize { 1 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for [T; 2] {
    fn bytes() -> usize { size_of::<[T; 2]>() }
    fn primitives() -> usize { 2 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for [T; 3] {
    fn bytes() -> usize { size_of::<[T; 3]>() }
    fn primitives() -> usize { 3 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for [T; 4] {
    fn bytes() -> usize { size_of::<[T; 4]>() }
    fn primitives() -> usize { 4 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for (T, T) {
    fn bytes() -> usize { size_of::<(T, T)>() }
    fn primitives() -> usize { 2 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for (T, T, T) {
    fn bytes() -> usize { size_of::<(T, T, T)>() }
    fn primitives() -> usize { 3 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexComponent> VertexComponent for (T, T, T, T) {
    fn bytes() -> usize { size_of::<(T, T, T, T)>() }
    fn primitives() -> usize { 4 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}


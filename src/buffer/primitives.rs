
//! Basic types used in all buffers

use gl;
use gl::types::*;
use std::mem::size_of;
use cable_math::{Vec2, Vec3, Vec4, Mat4};

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

/// Represents different gl buffer usage hints. Note that these are hints,
/// and drivers will not necessarily respect these.
///
/// The first part of the name indicates how frequently the data will be used:  
///
/// * Static - Data is set once and used often 
/// * Dynamic - Data is set frequently and used frequently
/// * Stream - Data is set once and used at most a few times
///
/// The second part indicates how it will be used:  
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

/// Represents a target to which a buffer can be bound
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
    Float         = gl::FLOAT,
    Int           = gl::INT,
    Short         = gl::SHORT,
    Byte          = gl::BYTE,
    UnsignedInt   = gl::UNSIGNED_INT,
    UnsignedShort = gl::UNSIGNED_SHORT,
    UnsignedByte  = gl::UNSIGNED_BYTE,
}
impl DataType {
    /// The number of bytes a single primitive of this type takes
    pub fn size(&self) -> usize {
        match *self {
            DataType::Float         => size_of::<GLfloat>(),
            DataType::Int           => size_of::<GLint>(),
            DataType::Short         => size_of::<GLshort>(),
            DataType::Byte          => size_of::<GLbyte>(),
            DataType::UnsignedInt   => size_of::<GLuint>(),
            DataType::UnsignedShort => size_of::<GLushort>(),
            DataType::UnsignedByte  => size_of::<GLubyte>(),
        }
    }

    /// Returns true if this type can be used to specify indices when using `glDrawElements` and
    /// similar functions. This returns true for `UnsignedInt`, `UnsignedShort` and `UnsignedByte`.
    /// In terms of rust types, this returns true for `u8`, `u16` and `u32`.
    pub fn indexable(&self) -> bool {
        match *self {
            DataType::UnsignedInt | DataType::UnsignedByte | DataType::UnsignedShort => true,
            _ => false,
        }
    }
}

/// Vertex buffers store a list of `Vertex`es (called vertices in proper
/// English) on the GPU. The difference between a `Vertex` and [`VertexData`]
/// is that a vertex contains information on how it interacts with a shader,
/// while you have to manually provide this information when using [`VertexData`].
///
/// This trait can be automatically derived for a struct with `#[derive(Vertex)]`. 
/// For this to work, all members of a struct need to implement [`VertexData`].
///
/// ```rust,ignore
/// extern crate gondola;
///
/// #[macro_use]
/// extern crate gondola_derive; // This crate provides custom derive
///
/// use gondola::buffer::Vertex; // We need to use the trait to derive it
///
/// #[derive(Vertex)]
/// struct Vert {
///     pos: (f32, f32, f32, f32),
///     uv: (f32, f32),
/// }
/// ```
///
/// [`VertexData`]: trait.VertexData.html
pub trait Vertex {
    fn bytes_per_vertex() -> usize;
    fn setup_attrib_pointers();
    fn gen_shader_input_decl() -> String;
}

/// This trait marks types which can be stored in a GPU buffer.  All fields of a 
/// struct need to implement this in order for `#[derive(Vertex)]` to work. 
///
/// Implemented for tuples, arrays and vectors of the types `GLfloat` (`f32`), 
/// `GLint` (`i32`), `GLuint` (`u32`), `GLshort` (`i16`), `GLushort` (`u16`), 
/// `GLbyte` (`i8`) and `GLubyte` (`u8`). Additionally the trait is implemented
/// for combinations of the former (e.g. arrays of tuples).
///
/// # Example - Implementing this trait for a custom type
/// ```rust
/// use gondola::buffer::{VertexData, DataType};
///
/// struct Triangle {
///     a: (f32, f32),
///     b: (f32, f32),
///     c: (f32, f32),
/// }
///
/// impl VertexData for Triangle {
///     fn primitives() -> usize { 6 } // 3 tuples of 2 primitives
///     fn data_type() -> DataType { DataType::Float }
/// }
/// ```
pub trait VertexData: Sized {
    /// The total number of bytes one of these components takes.
    fn bytes() -> usize {
        size_of::<Self>()
    }

    /// The total number of primitives one of these components provides (e.g. 4 for a `Vec4<T>`).
    fn primitives() -> usize;

    /// The type of primitives this component provides.
    fn data_type() -> DataType;

    /// Generates the type that would be used to represent this component in a
    /// glsl shader
    fn get_glsl_type() -> String {
        let primitives = <Self as VertexData>::primitives();
        let data_type = <Self as VertexData>::data_type();

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
            panic!("Invalid VertexData: {} primitives of type {:?}", primitives, data_type);
        }

        result
    }
}

// Implementations for VertexData

macro_rules! impl_vertex_data {
    ($primitive:ty, $data_type:expr) => {
        impl VertexData for $primitive {
            fn primitives() -> usize { 1 }
            fn data_type() -> DataType { $data_type }
        }
    }
}
impl_vertex_data!(GLfloat, DataType::Float);
impl_vertex_data!(GLint, DataType::Int);
impl_vertex_data!(GLuint, DataType::UnsignedInt);
impl_vertex_data!(GLbyte, DataType::Byte);
impl_vertex_data!(GLubyte, DataType::UnsignedByte);
impl_vertex_data!(GLshort, DataType::Int);
impl_vertex_data!(GLushort, DataType::UnsignedInt);

// Recursive generics woo!!!
impl<T: VertexData> VertexData for Mat4<T> {
    fn primitives() -> usize { 16 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for Vec2<T> {
    fn primitives() -> usize { 2 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for Vec3<T> {
    fn primitives() -> usize { 3 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for Vec4<T> {
    fn primitives() -> usize { 4 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for [T; 1] {
    fn primitives() -> usize { 1 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for [T; 2] {
    fn primitives() -> usize { 2 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for [T; 3] {
    fn primitives() -> usize { 3 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for [T; 4] {
    fn primitives() -> usize { 4 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for (T, T) {
    fn primitives() -> usize { 2 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for (T, T, T) {
    fn primitives() -> usize { 3 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}
impl<T: VertexData> VertexData for (T, T, T, T) {
    fn primitives() -> usize { 4 * T::primitives() }
    fn data_type() -> DataType { T::data_type() }
}


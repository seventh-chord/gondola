
//! Basic types used in all buffers

use gl;
use gl::types::*;
use std::mem::size_of;
use cable_math::{Vec2, Vec3, Vec4, Mat4};

/// Represents different types of primitives which can be drawn on the GPU.
#[repr(u32)] // GLenum is u32
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

impl PrimitiveMode {
    /// Returns the base primitive for this type. Either `gl::POINTS`, `gl::LINES` or
    /// `gl::TRIANGLES`.
    pub fn gl_base_primitive(&self) -> GLenum {
        match *self {
            PrimitiveMode::Points
                => gl::POINTS,

            PrimitiveMode::LineStrip | PrimitiveMode::LineLoop | PrimitiveMode::Lines |
            PrimitiveMode::LineStripAdjacency | PrimitiveMode::LinesAdjacency
                => gl::LINES,

            PrimitiveMode::TriangleStrip | PrimitiveMode::TriangleFan | PrimitiveMode::Triangles |
            PrimitiveMode::TriangleStripAdjacency | PrimitiveMode::TrianglesAdjacency
                => gl::TRIANGLES,
        }
    }
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

/// This trait is used to mark types which are OpenGL primitives. You should not implement this
/// trait yourself. If you want to use a custom type in a [`VertexBuffer`], implement [`Vertex`].
/// Similarly, if you want to use a custom type in a [`PrimitiveBuffer`], implement [`VertexData`]
/// for it.
///
/// This trait is implemented for all the basic OpenGL primitives: `GLfloat`, `GLint`, `GLshort`,
/// `GLbyte`, `GLuint`, `GLushort` and `GLubyte`, which correspond to the rust primitives `f32`,
/// `i32`, `i16`, `i8`, `u32`, `u16` and `u8`.
///
/// [`VertexBuffer`]:    struct.VertexBuffer.html
/// [`PrimitiveBuffer`]: struct.PrimitiveBuffer.html
/// [`Vertex`]:          trait.Vertex.html
/// [`VertexData`]:      trait.VertexData.html
pub trait GlPrimitive: Sized {
    fn glsl_scalar_name() -> &'static str;
    fn glsl_vec_name() -> &'static str;
    fn rust_name() -> &'static str;
    fn gl_name() -> &'static str;
    fn gl_enum() -> GLenum;
    fn is_integer() -> bool;
}

impl GlPrimitive for GLfloat {
    fn glsl_scalar_name() -> &'static str { "float" }
    fn glsl_vec_name() -> &'static str    { "vec" }
    fn rust_name() -> &'static str { "f32" }
    fn gl_name() -> &'static str   { "GLfloat" }
    fn gl_enum() -> GLenum { gl::FLOAT }
    fn is_integer() -> bool { false }
}
impl GlPrimitive for GLint {
    fn glsl_scalar_name() -> &'static str { "int" }
    fn glsl_vec_name() -> &'static str    { "ivec" }
    fn rust_name() -> &'static str { "i32" }
    fn gl_name() -> &'static str   { "GLint" }
    fn gl_enum() -> GLenum { gl::INT }
    fn is_integer() -> bool { true }
}
impl GlPrimitive for GLshort {
    fn glsl_scalar_name() -> &'static str { "int" }
    fn glsl_vec_name() -> &'static str    { "ivec" }
    fn rust_name() -> &'static str { "i16" }
    fn gl_name() -> &'static str   { "GLshort" }
    fn gl_enum() -> GLenum { gl::SHORT }
    fn is_integer() -> bool { true }
}
impl GlPrimitive for GLbyte {
    fn glsl_scalar_name() -> &'static str { "int" }
    fn glsl_vec_name() -> &'static str    { "ivec" }
    fn rust_name() -> &'static str { "i8" }
    fn gl_name() -> &'static str   { "GLbyte" }
    fn gl_enum() -> GLenum { gl::BYTE }
    fn is_integer() -> bool { true }
}
impl GlPrimitive for GLuint {
    fn glsl_scalar_name() -> &'static str { "uint" }
    fn glsl_vec_name() -> &'static str    { "uvec" }
    fn rust_name() -> &'static str { "u32" }
    fn gl_name() -> &'static str   { "GLuint" }
    fn gl_enum() -> GLenum { gl::UNSIGNED_INT }
    fn is_integer() -> bool { true }
}
impl GlPrimitive for GLushort {
    fn glsl_scalar_name() -> &'static str { "uint" }
    fn glsl_vec_name() -> &'static str    { "uvec" }
    fn rust_name() -> &'static str { "u16" }
    fn gl_name() -> &'static str   { "GLushort" }
    fn gl_enum() -> GLenum { gl::UNSIGNED_SHORT }
    fn is_integer() -> bool { true }
}
impl GlPrimitive for GLubyte {
    fn glsl_scalar_name() -> &'static str { "uint" }
    fn glsl_vec_name() -> &'static str    { "uvec" }
    fn rust_name() -> &'static str { "u8" }
    fn gl_name() -> &'static str   { "GLubyte" }
    fn gl_enum() -> GLenum { gl::UNSIGNED_BYTE }
    fn is_integer() -> bool { true }
}

/// This trait is used to mark types which can be used as indices in e.g. a element/index buffer.
/// You should not implement this trait yourself.
///
/// This trait is implemented for `GLuint`, `GLushort` and `GLubyte`, which correspond to `u32`,
/// `u16` and `u8`.
pub trait GlIndex: Sized + GlPrimitive {}

impl GlIndex for GLuint {}
impl GlIndex for GLushort {}
impl GlIndex for GLubyte {}

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
pub trait Vertex: Sized {
    fn bytes_per_vertex() -> usize;
    fn setup_attrib_pointers(divisor: usize);

    fn gen_shader_input_decl(name_prefix: &str) -> String;
    fn gen_transform_feedback_outputs(name_prefix: &str) -> Vec<String>;
    fn gen_transform_feedback_decl(name_prefix: &str) -> String;
}

/// This trait marks types which can be stored in a GPU buffer.  All fields of a 
/// struct need to implement this in order for `#[derive(Vertex)]` to work. Note
/// that any struct that implements this trait should only contains fields of a
/// single type.
///
/// By default this trait is implemented for tuples, arrays and vectors of all 
/// [`GlPrimitives`].
///
/// [`GlPrimitives`]: trait.GlPrimitives.html
///
/// # Example - Implementing this trait for a custom type
/// ```rust
/// use gondola::buffer::VertexData;
///
/// #[repr(C)]
/// struct Point {
///     a: (f32, f32),
/// }
///
/// impl VertexData for Point {
///     type Primitive = f32;
/// }
/// ```
pub trait VertexData: Sized {
    type Primitive: GlPrimitive;

    /// The total number of bytes one of these components takes.
    fn bytes() -> usize {
        size_of::<Self>()
    }

    /// The total number of primitives one of these components provides (e.g. 4 for a `Vec4<T>`).
    fn primitives() -> usize {
        assert_eq!(size_of::<Self>() % size_of::<Self::Primitive>(), 0);

        size_of::<Self>() / size_of::<Self::Primitive>()
    }

    /// Generates the type that would be used to represent this component in a
    /// glsl shader
    fn get_glsl_type() -> String {
        let primitives = <Self as VertexData>::primitives();

        let mut result = String::with_capacity(6);

        if primitives == 1 {
            result.push_str(Self::Primitive::glsl_scalar_name());
        } else if primitives > 1 && primitives <= 4 {
            result.push_str(Self::Primitive::glsl_vec_name());
            result.push_str(&primitives.to_string());
        }

        if result.is_empty() {
            panic!("Invalid VertexData: {} primitives of type {}/{} are not supported for glsl yet (At {}:{})", 
                   primitives,
                   Self::Primitive::rust_name(), Self::Primitive::gl_name(),
                   file!(), line!());
        }

        result
    }
}


// Implementations for VertexData:
impl<T: GlPrimitive> VertexData for T {
    type Primitive = T; 
}

impl<T: VertexData> VertexData for Mat4<T> {
    type Primitive = T::Primitive;
}

impl<T: VertexData> VertexData for Vec2<T> {
    type Primitive = T::Primitive;
}
impl<T: VertexData> VertexData for Vec3<T> {
    type Primitive = T::Primitive;
}
impl<T: VertexData> VertexData for Vec4<T> {
    type Primitive = T::Primitive;
}

impl<T: VertexData> VertexData for (T, T) {
    type Primitive = T::Primitive;
}
impl<T: VertexData> VertexData for (T, T, T) {
    type Primitive = T::Primitive;
}
impl<T: VertexData> VertexData for (T, T, T, T) {
    type Primitive = T::Primitive;
}

macro_rules! impl_array { ($count:expr) => {
    impl<T: VertexData> VertexData for [T; $count] {
        type Primitive = T::Primitive;
    }
} }
impl_array!(1);  impl_array!(2);  impl_array!(3);  impl_array!(4);  impl_array!(5);  impl_array!(6);
impl_array!(7);  impl_array!(8);  impl_array!(9);  impl_array!(10); impl_array!(11); impl_array!(12);
impl_array!(13); impl_array!(14); impl_array!(15); impl_array!(16); impl_array!(17); impl_array!(18);
impl_array!(19); impl_array!(20); impl_array!(21); impl_array!(22); impl_array!(23); impl_array!(24);
impl_array!(25); impl_array!(26); impl_array!(27); impl_array!(28); impl_array!(29); impl_array!(30);
impl_array!(31); impl_array!(32); impl_array!(33); impl_array!(34); impl_array!(35); impl_array!(36);

/// Reperesents the data needed for a call to `gl::EnableVertexAttribArray`,
/// `gl::VertexAttribPointer` and `gl::VertexAttribDivisor`. This is mainly
/// intended for internal usage and when deriving [`Vertex`].
///
/// [`Vertex`]: struct.Vertex.html
#[derive(Debug, Clone)]
pub struct AttribBinding {
    /// The vertex attribute to which this binding will serve values.
    pub index: usize,
    /// The number of primitives per vertex this attribute will serve to shaders.
    pub primitives: usize,
    /// The type of primitives which this attribute will serve to shaders. Should be a constant
    /// defined by OpenGL.
    pub primitive_type: u32,
    /// If set to true, integer types will be parsed as floats and mapped to the range `0.0..1.0`
    /// for unsigned integers and `-1.0..1.0` for signed integers.
    pub normalized: bool,
    /// If set to true, `glVertexAttribIPointer` is used instead of `glVertexAttribPointer`. This
    /// is only valid if `primitive_tpye` is a integer primitive. If this is set to true,
    /// `normalized` is ignored.
    pub integer: bool,
    /// The distance, in bytes, between each set of primitives
    pub stride: usize,
    /// The index, in bytes, of the first byte of data
    pub offset: usize,

    /// The number of vertices from other sources for which this source will be used. For example,
    /// if set to 3 every set of three vertices will use one instance from this source.
    pub divisor: usize,
}

impl AttribBinding {
    /// Calls `gl::EnableVertexAttribArray`, `gl::VertexAttribPointer` and `gl::VertexAttribDivisor`.
    pub fn enable(&self) {
        use gl;
        use gl::types::*;

        unsafe {
            gl::EnableVertexAttribArray(self.index as GLuint);

            if self.integer {
                gl::VertexAttribIPointer(
                    self.index as GLuint, self.primitives as GLint,
                    self.primitive_type as GLenum, self.stride as GLsizei, 
                    self.offset as *const GLvoid
                );
            } else {
                gl::VertexAttribPointer(
                    self.index as GLuint, self.primitives as GLint,
                    self.primitive_type as GLenum, self.normalized as GLboolean,
                    self.stride as GLsizei, self.offset as *const GLvoid
                );
            }

            gl::VertexAttribDivisor(self.index as GLuint, self.divisor as GLuint);
        }
    }
}

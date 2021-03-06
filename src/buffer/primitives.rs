
//! Basic types used in all buffers

use std::mem;

use gl;
use gl::types::*;

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
    const GLSL_SCALAR_NAME: &'static str;
    const GLSL_VEC_NAME:    &'static str;
    const RUST_NAME:        &'static str;
    const GL_NAME:          &'static str;

    const GL_ENUM: GLenum;
    const IS_INTEGER: bool;

    /// This sets a constant value to a given vertex attribute
    fn set_as_vertex_attrib(&self, _location: usize) {
        panic!("Can't set {}/{} as vertex attributes", Self::RUST_NAME, Self::GLSL_SCALAR_NAME);
    }
}

impl GlPrimitive for GLfloat {
    const GLSL_SCALAR_NAME: &'static str = "float";
    const GLSL_VEC_NAME:    &'static str = "vec";
    const RUST_NAME:        &'static str = "f32";
    const GL_NAME:          &'static str = "GLfloat";

    const GL_ENUM: GLenum  = gl::FLOAT;
    const IS_INTEGER: bool = false;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttrib1f(location as GLuint, *self) }
    }
}
impl GlPrimitive for GLint {
    const GLSL_SCALAR_NAME: &'static str = "int";
    const GLSL_VEC_NAME:    &'static str    = "ivec";
    const RUST_NAME:        &'static str = "i32";
    const GL_NAME:          &'static str = "GLint";

    const GL_ENUM: GLenum  = gl::INT;
    const IS_INTEGER: bool = true;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttribI1i(location as GLuint, *self) }
    }
}
impl GlPrimitive for GLshort {
    const GLSL_SCALAR_NAME: &'static str = "int";
    const GLSL_VEC_NAME:    &'static str    = "ivec";
    const RUST_NAME:        &'static str = "i16";
    const GL_NAME:          &'static str = "GLshort";

    const GL_ENUM: GLenum  = gl::SHORT;
    const IS_INTEGER: bool = true;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttrib1s(location as GLuint, *self) }
    }
}
impl GlPrimitive for GLbyte {
    const GLSL_SCALAR_NAME: &'static str = "int";
    const GLSL_VEC_NAME:    &'static str    = "ivec";
    const RUST_NAME:        &'static str = "i8";
    const GL_NAME:          &'static str = "GLbyte";

    const GL_ENUM: GLenum  = gl::BYTE;
    const IS_INTEGER: bool = true;
}
impl GlPrimitive for GLuint {
    const GLSL_SCALAR_NAME: &'static str = "uint";
    const GLSL_VEC_NAME:    &'static str    = "uvec";
    const RUST_NAME:        &'static str = "u32";
    const GL_NAME:          &'static str = "GLuint";

    const GL_ENUM: GLenum  = gl::UNSIGNED_INT;
    const IS_INTEGER: bool = true;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttribI1ui(location as GLuint, *self) }
    }
}
impl GlPrimitive for GLushort {
    const GLSL_SCALAR_NAME: &'static str = "uint";
    const GLSL_VEC_NAME:    &'static str    = "uvec";
    const RUST_NAME:        &'static str = "u16";
    const GL_NAME:          &'static str = "GLushort";

    const GL_ENUM: GLenum  = gl::UNSIGNED_SHORT;
    const IS_INTEGER: bool = true;
}
impl GlPrimitive for GLubyte {
    const GLSL_SCALAR_NAME: &'static str = "uint";
    const GLSL_VEC_NAME:    &'static str    = "uvec";
    const RUST_NAME:        &'static str = "u8";
    const GL_NAME:          &'static str = "GLubyte";

    const GL_ENUM: GLenum  = gl::UNSIGNED_BYTE;
    const IS_INTEGER: bool = true;
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
    fn setup_attrib_pointers(divisor: usize);
    fn gen_shader_input_decl(name_prefix: &str) -> String;
    fn gen_transform_feedback_outputs(name_prefix: &str) -> Vec<String>;
    fn gen_transform_feedback_decl(name_prefix: &str) -> String;
    fn set_as_vertex_attrib(&self);
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
// TODO (Morten, 09.12.17) This trait (and all traits here for that matter) should probably be
// marked as unsafe, to prevent people from implementing them!
pub trait VertexData: Sized {
    type Primitive: GlPrimitive;

    /// The total number of primitives one of these components provides (e.g. 4 for a `Vec4<T>`).
    fn primitives() -> usize {
        assert_eq!(mem::size_of::<Self>() % mem::size_of::<Self::Primitive>(), 0);

        mem::size_of::<Self>() / mem::size_of::<Self::Primitive>()
    }

    /// Generates the type that would be used to represent this component in a
    /// glsl shader
    fn get_glsl_type() -> String {
        let primitives = <Self as VertexData>::primitives();

        let mut result = String::with_capacity(6);

        if primitives == 1 {
            result.push_str(Self::Primitive::GLSL_SCALAR_NAME);
        } else if primitives > 1 && primitives <= 4 {
            result.push_str(Self::Primitive::GLSL_VEC_NAME);
            result.push_str(&primitives.to_string());
        }

        if result.is_empty() {
            panic!(
                "Invalid VertexData: {} primitives of type {}/{} are not supported for glsl", 
                primitives,
                Self::Primitive::RUST_NAME, Self::Primitive::GL_NAME,
            );
        }

        result
    }

    fn set_as_vertex_attrib(&self, _location: usize) {
        panic!(
            "Not implemented. Probably can't set {} primitives of type {}/{} as a vertex attribute",
            <Self as VertexData>::primitives(),
            Self::Primitive::RUST_NAME, Self::Primitive::GL_NAME,
        );
    }
}


// Implementations for VertexData:
impl<T: GlPrimitive> VertexData for T {
    type Primitive = T; 

    fn set_as_vertex_attrib(&self, location: usize) {
        T::set_as_vertex_attrib(&self, location);
    }
}

impl VertexData for Mat4<f32> {
    type Primitive = f32;
}

impl VertexData for Vec2<f32> {
    type Primitive = f32;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttrib2f(location as GLuint, self.x, self.y) }
    }
}
impl VertexData for Vec3<f32> {
    type Primitive = f32;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttrib3f(location as GLuint, self.x, self.y, self.z) }
    }
}
impl VertexData for Vec4<f32> {
    type Primitive = f32;

    fn set_as_vertex_attrib(&self, location: usize) {
        unsafe { gl::VertexAttrib4f(location as GLuint, self.x, self.y, self.z, self.w) }
    }
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

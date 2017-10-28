
use std::fmt;

use gl;
use gl::types::*;
use cable_math::{Mat4, Vec2, Vec3, Vec4};

pub struct UniformBinding {
    pub name: String,
    pub location: GLint,
    pub kind: UniformKind,
}

/// Everything which implements this trait can be stured into the uniform value of a shader.
pub trait UniformValue: Sized {
    const KIND: UniformKind;

    unsafe fn set_uniform(data: &Self, location: GLint); 
    unsafe fn set_uniform_slice(slice: &[Self], location: GLint);
}

#[repr(u32)] // GLenum is u32
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UniformKind {
    // NB (Morten, 28.10.17) This list is not complete, it only contains the types we need at the
    // moment! There is a table here (http://docs.gl/gl3/glGetActiveUniform) which contains a list
    // of all possible values!

    F32      = gl::FLOAT,
    VEC2_F32 = gl::FLOAT_VEC2,
    VEC3_F32 = gl::FLOAT_VEC3,
    VEC4_F32 = gl::FLOAT_VEC4,
    MAT4_F32 = gl::FLOAT_MAT4,

    I32      = gl::INT,
    VEC2_I32 = gl::INT_VEC2,
    VEC3_I32 = gl::INT_VEC3,
    VEC4_I32 = gl::INT_VEC4,

    U32      = gl::UNSIGNED_INT,
    VEC2_U32 = gl::UNSIGNED_INT_VEC2,
    VEC3_U32 = gl::UNSIGNED_INT_VEC3,
    VEC4_U32 = gl::UNSIGNED_INT_VEC4,
}

// Implementations for vectors and matricies
impl UniformValue for Vec2<f32> { 
    const KIND: UniformKind = UniformKind::VEC2_F32;

    unsafe fn set_uniform(vec: &Vec2<f32>, location: GLint) {
        gl::Uniform2f(location, vec.x, vec.y); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec2<f32>], location: GLint) {
        gl::Uniform2fv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLfloat); 
    }
}

impl UniformValue for Vec2<i32> { 
    const KIND: UniformKind = UniformKind::VEC2_I32;

    unsafe fn set_uniform(vec: &Vec2<i32>, location: GLint) {
        gl::Uniform2i(location, vec.x, vec.y); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec2<i32>], location: GLint) {
        gl::Uniform2iv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLint); 
    } 
}

impl UniformValue for Vec2<u32> {
    const KIND: UniformKind = UniformKind::VEC2_U32;

    unsafe fn set_uniform(vec: &Vec2<u32>, location: GLint) {
        gl::Uniform2ui(location, vec.x, vec.y); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec2<u32>], location: GLint) {
        gl::Uniform2uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLuint); 
    } 
}

impl UniformValue for Vec3<f32> { 
    const KIND: UniformKind = UniformKind::VEC3_F32;

    unsafe fn set_uniform(vec: &Vec3<f32>, location: GLint) {
        gl::Uniform3f(location, vec.x, vec.y, vec.z); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec3<f32>], location: GLint) {
        gl::Uniform3fv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLfloat); 
    }
}

impl UniformValue for Vec3<i32> { 
    const KIND: UniformKind = UniformKind::VEC3_I32;

    unsafe fn set_uniform(vec: &Vec3<i32>, location: GLint) {
        gl::Uniform3i(location, vec.x, vec.y, vec.z); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec3<i32>], location: GLint) {
        gl::Uniform3iv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLint); 
    } 
}

impl UniformValue for Vec3<u32> {
    const KIND: UniformKind = UniformKind::VEC3_U32;

    unsafe fn set_uniform(vec: &Vec3<u32>, location: GLint) {
        gl::Uniform3ui(location, vec.x, vec.y, vec.z); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec3<u32>], location: GLint) {
        gl::Uniform3uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLuint); 
    } 
}

impl UniformValue for Vec4<f32> { 
    const KIND: UniformKind = UniformKind::VEC4_F32;

    unsafe fn set_uniform(vec: &Vec4<f32>, location: GLint) {
        gl::Uniform4f(location, vec.x, vec.y, vec.z, vec.w); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec4<f32>], location: GLint) {
        gl::Uniform4fv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLfloat); 
    }
}

impl UniformValue for Vec4<i32> { 
    const KIND: UniformKind = UniformKind::VEC4_I32;

    unsafe fn set_uniform(vec: &Vec4<i32>, location: GLint) {
        gl::Uniform4i(location, vec.x, vec.y, vec.z, vec.w); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec4<i32>], location: GLint) {
        gl::Uniform4iv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLint); 
    } 
}

impl UniformValue for Vec4<u32> {
    const KIND: UniformKind = UniformKind::VEC4_U32;

    unsafe fn set_uniform(vec: &Vec4<u32>, location: GLint) {
        gl::Uniform4ui(location, vec.x, vec.y, vec.z, vec.w); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec4<u32>], location: GLint) {
        gl::Uniform4uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLuint); 
    } 
}

impl UniformValue for Mat4<f32> {
    const KIND: UniformKind = UniformKind::MAT4_F32;

    unsafe fn set_uniform(mat: &Mat4<f32>, location: GLint) {
        gl::UniformMatrix4fv(location, 1, false as GLboolean, &(mat.a11) as *const GLfloat); 
    }

    unsafe fn set_uniform_slice(slice: &[Mat4<f32>], location: GLint) {
        gl::UniformMatrix4fv(location, slice.len() as GLsizei, false as GLboolean, slice.as_ptr() as *const GLfloat); 
    }
}

// Implementations for f32, i32 and u32 single values and tuples.
impl UniformValue for f32 {
    const KIND: UniformKind = UniformKind::F32;

    unsafe fn set_uniform(value: &f32, location: GLint) {
        gl::Uniform1f(location, *value); 
    }

    unsafe fn set_uniform_slice(slice: &[f32], location: GLint) {
        gl::Uniform1fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for (f32, f32) {
    const KIND: UniformKind = UniformKind::VEC2_F32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform2f(location, value.0, value.1); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform2fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for (f32, f32, f32) {
    const KIND: UniformKind = UniformKind::VEC3_F32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform3f(location, value.0, value.1, value.2); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform3fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for (f32, f32, f32, f32) {
    const KIND: UniformKind = UniformKind::VEC4_F32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform4f(location, value.0, value.1, value.2, value.3); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform4fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for i32 {
    const KIND: UniformKind = UniformKind::I32;

    unsafe fn set_uniform(value: &i32, location: GLint) {
        gl::Uniform1i(location, *value); 
    }

    unsafe fn set_uniform_slice(slice: &[i32], location: GLint) {
        gl::Uniform1iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for (i32, i32) {
    const KIND: UniformKind = UniformKind::VEC2_I32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform2i(location, value.0, value.1); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform2iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for (i32, i32, i32) {
    const KIND: UniformKind = UniformKind::VEC3_I32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform3i(location, value.0, value.1, value.2); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform3iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for (i32, i32, i32, i32) {
    const KIND: UniformKind = UniformKind::VEC4_I32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform4i(location, value.0, value.1, value.2, value.3); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform4iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for u32 {
    const KIND: UniformKind = UniformKind::U32;

    unsafe fn set_uniform(value: &u32, location: GLint) {
        gl::Uniform1ui(location, *value); 
    }

    unsafe fn set_uniform_slice(slice: &[u32], location: GLint) {
        gl::Uniform1uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

impl UniformValue for (u32, u32) {
    const KIND: UniformKind = UniformKind::VEC2_U32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform2ui(location, value.0, value.1); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform2uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

impl UniformValue for (u32, u32, u32) {
    const KIND: UniformKind = UniformKind::VEC3_U32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform3ui(location, value.0, value.1, value.2); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform3uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

impl UniformValue for (u32, u32, u32, u32) {
    const KIND: UniformKind = UniformKind::VEC4_U32;

    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform4ui(location, value.0, value.1, value.2, value.3); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform4uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}


impl fmt::Display for UniformKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::UniformKind::*;

        let name = match *self {
            F32      => "f32",
            VEC2_F32 => "Vec2<f32>",
            VEC3_F32 => "Vec3<f32>",
            VEC4_F32 => "Vec4<f32>",
            MAT4_F32 => "Mat4<f32>",

            I32      => "i32",
            VEC2_I32 => "Vec2<i32>",
            VEC3_I32 => "Vec3<i32>",
            VEC4_I32 => "Vec4<i32>",

            U32      => "u32",
            VEC2_U32 => "Vec2<u32>",
            VEC3_U32 => "Vec3<u32>",
            VEC4_U32 => "Vec4<u32>",
        };

        f.write_str(name)
    }
}

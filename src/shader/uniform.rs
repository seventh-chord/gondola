
use gl;
use gl::types::*;
use cable_math::{Mat4, Vec2, Vec3, Vec4};

pub struct UniformBinding {
    pub name: String,
    pub location: GLint,
    pub kind: GLenum,
}

/// Everything which implements this trait can be stured into the uniform value of a shader.
pub trait UniformValue: Sized {
    unsafe fn set_uniform(data: &Self, location: GLint); 
    unsafe fn set_uniform_slice(slice: &[Self], location: GLint);
}

// Implementations for vectors and matricies
impl UniformValue for Vec2<f32> { 
    unsafe fn set_uniform(vec: &Vec2<f32>, location: GLint) {
        gl::Uniform2f(location, vec.x, vec.y); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec2<f32>], location: GLint) {
        gl::Uniform2fv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLfloat); 
    }
}

impl UniformValue for Vec2<i32> { 
    unsafe fn set_uniform(vec: &Vec2<i32>, location: GLint) {
        gl::Uniform2i(location, vec.x, vec.y); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec2<i32>], location: GLint) {
        gl::Uniform2iv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLint); 
    } 
}

impl UniformValue for Vec2<u32> {
    unsafe fn set_uniform(vec: &Vec2<u32>, location: GLint) {
        gl::Uniform2ui(location, vec.x, vec.y); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec2<u32>], location: GLint) {
        gl::Uniform2uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLuint); 
    } 
}

impl UniformValue for Vec3<f32> { 
    unsafe fn set_uniform(vec: &Vec3<f32>, location: GLint) {
        gl::Uniform3f(location, vec.x, vec.y, vec.z); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec3<f32>], location: GLint) {
        gl::Uniform3fv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLfloat); 
    }
}

impl UniformValue for Vec3<i32> { 
    unsafe fn set_uniform(vec: &Vec3<i32>, location: GLint) {
        gl::Uniform3i(location, vec.x, vec.y, vec.z); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec3<i32>], location: GLint) {
        gl::Uniform3iv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLint); 
    } 
}

impl UniformValue for Vec3<u32> {
    unsafe fn set_uniform(vec: &Vec3<u32>, location: GLint) {
        gl::Uniform3ui(location, vec.x, vec.y, vec.z); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec3<u32>], location: GLint) {
        gl::Uniform3uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLuint); 
    } 
}

impl UniformValue for Vec4<f32> { 
    unsafe fn set_uniform(vec: &Vec4<f32>, location: GLint) {
        gl::Uniform4f(location, vec.x, vec.y, vec.z, vec.w); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec4<f32>], location: GLint) {
        gl::Uniform4fv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLfloat); 
    }
}

impl UniformValue for Vec4<i32> { 
    unsafe fn set_uniform(vec: &Vec4<i32>, location: GLint) {
        gl::Uniform4i(location, vec.x, vec.y, vec.z, vec.w); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec4<i32>], location: GLint) {
        gl::Uniform4iv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLint); 
    } 
}

impl UniformValue for Vec4<u32> {
    unsafe fn set_uniform(vec: &Vec4<u32>, location: GLint) {
        gl::Uniform4ui(location, vec.x, vec.y, vec.z, vec.w); 
    }

    unsafe fn set_uniform_slice(slice: &[Vec4<u32>], location: GLint) {
        gl::Uniform4uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const GLuint); 
    } 
}

impl UniformValue for Mat4<f32> {
    unsafe fn set_uniform(mat: &Mat4<f32>, location: GLint) {
        gl::UniformMatrix4fv(location, 1, false as GLboolean, &(mat.a11) as *const GLfloat); 
    }

    unsafe fn set_uniform_slice(slice: &[Mat4<f32>], location: GLint) {
        gl::UniformMatrix4fv(location, slice.len() as GLsizei, false as GLboolean, slice.as_ptr() as *const GLfloat); 
    }
}

// Implementations for f32, i32 and u32 single values and tuples.
impl UniformValue for f32 {
    unsafe fn set_uniform(value: &f32, location: GLint) {
        gl::Uniform1f(location, *value); 
    }

    unsafe fn set_uniform_slice(slice: &[f32], location: GLint) {
        gl::Uniform1fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for (f32, f32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform2f(location, value.0, value.1); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform2fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for (f32, f32, f32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform3f(location, value.0, value.1, value.2); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform3fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for (f32, f32, f32, f32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform4f(location, value.0, value.1, value.2, value.3); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform4fv(location, slice.len() as GLsizei, slice.as_ptr() as *const f32); 
    }
}

impl UniformValue for i32 {
    unsafe fn set_uniform(value: &i32, location: GLint) {
        gl::Uniform1i(location, *value); 
    }

    unsafe fn set_uniform_slice(slice: &[i32], location: GLint) {
        gl::Uniform1iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for (i32, i32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform2i(location, value.0, value.1); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform2iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for (i32, i32, i32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform3i(location, value.0, value.1, value.2); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform3iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for (i32, i32, i32, i32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform4i(location, value.0, value.1, value.2, value.3); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform4iv(location, slice.len() as GLsizei, slice.as_ptr() as *const i32); 
    }
}

impl UniformValue for u32 {
    unsafe fn set_uniform(value: &u32, location: GLint) {
        gl::Uniform1ui(location, *value); 
    }

    unsafe fn set_uniform_slice(slice: &[u32], location: GLint) {
        gl::Uniform1uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

impl UniformValue for (u32, u32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform2ui(location, value.0, value.1); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform2uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

impl UniformValue for (u32, u32, u32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform3ui(location, value.0, value.1, value.2); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform3uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

impl UniformValue for (u32, u32, u32, u32) {
    unsafe fn set_uniform(value: &Self, location: GLint) {
        gl::Uniform4ui(location, value.0, value.1, value.2, value.3); 
    }

    unsafe fn set_uniform_slice(slice: &[Self], location: GLint) {
        gl::Uniform4uiv(location, slice.len() as GLsizei, slice.as_ptr() as *const u32); 
    }
}

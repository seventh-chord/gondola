
use gl;
use gl::types::*;

#[allow(non_camel_case_types, dead_code)]
pub enum TextureFormat {
    RGBA_F32,
    RGBA_F16,
    RGB_F32,
    RGB_F16,
    R_F32,
    R_F16,

    RGBA_8,
    RGB_8,
    R_8
}

impl TextureFormat {
    pub fn get_gl_enum(&self) -> GLenum {
        match *self {
            TextureFormat::RGBA_F32 => gl::RGBA32F,
            TextureFormat::RGBA_F16 => gl::RGBA16F,
            TextureFormat::RGB_F32  => gl::RGBA32F,
            TextureFormat::RGB_F16  => gl::RGB16F,
            TextureFormat::R_F32    => gl::R32F,
            TextureFormat::R_F16    => gl::R16F,
            TextureFormat::RGBA_8   => gl::RGBA8,
            TextureFormat::RGB_8    => gl::RGB8,
            TextureFormat::R_8      => gl::R8,
        }
    }
}

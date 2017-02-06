
use gl;

#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone)]
pub enum TextureFormat {
    RGBA_F32 = gl::RGBA32F as isize,
    RGBA_F16 = gl::RGBA16F as isize,
    RGB_F32  = gl::RGB32F as isize,
    RGB_F16  = gl::RGB16F as isize,
    R_F32    = gl::R32F as isize,
    R_F16    = gl::R16F as isize,

    RGBA_8   = gl::RGBA8 as isize,
    RGB_8    = gl::RGB8 as isize,
    R_8      = gl::R8 as isize,
}


use image;
use gl;
use gl::types::*;
use std::fmt;
use std::path::{ Path, PathBuf };
use std::error::Error;

/// Attempts to load a texture from the given path
pub fn load(path: &Path) -> Result<Texture, image::ImageError> {
    let image = image::open(path)?;
    let image = match image {
        image::DynamicImage::ImageRgba8(image) => image,
        other => other.to_rgba() // Convert other formats to RGBA
    }; 

    // Note that image dereferences to &[u8]
    let mut texture = Texture::with_data(&image,
                                         image.width(), image.height(),
                                         TextureFormat::RGBA_8);
    texture.source_file = Some(PathBuf::from(path));
    Ok(texture)
}

#[derive(Debug)]
pub struct Texture {
    source_file: Option<PathBuf>, // If this texture did not originate from a file, this will be None 
    texture: GLuint,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    pub fn with_data(data: &[u8], width: u32, height: u32, format: TextureFormat) -> Texture {
        let mut texture = 0;

        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(gl::TEXTURE_2D, 0, // Mipmap level
                           format as GLint, // Internal format
                           width as GLsizei, height as GLsizei, 0, // Size and border
                           format.unsized_format(), // Data format
                           gl::UNSIGNED_BYTE, data.as_ptr() as *const GLvoid);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        }

        Texture {
            source_file: None,
            texture: texture,
            format: format,
            width: width,
            height: height,
        }
    }

    /// Binds this texture to the given texture unit
    pub fn bind(&self, unit: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit as GLenum);
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
        }
    }
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, Copy, Clone)]
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
impl TextureFormat {
    /// Retrives the unsized version of this format
    fn unsized_format(&self) -> GLenum {
        match *self {
            TextureFormat::RGBA_F32 | TextureFormat::RGBA_F16 | TextureFormat::RGBA_8 => gl::RGBA,
            TextureFormat::RGB_F32 | TextureFormat::RGB_F16 | TextureFormat::RGB_8 => gl::RGB,
            TextureFormat::R_F32 | TextureFormat::R_F16 | TextureFormat::R_8 => gl::RED
        }
    }
}


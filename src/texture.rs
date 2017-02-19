
use image;
use gl;
use gl::types::*;
use std::path::{ Path, PathBuf };

#[derive(Debug)]
pub struct Texture {
    source_file: Option<PathBuf>, // If this texture did not originate from a file, this will be None 
    texture: GLuint,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl Texture {
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

    /// Creates a new texture with the given data. Usually the `Texture::load(path)` function
    /// should be used instead.
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

    /// Sets the filter that is applied when this texture is rendered at a size larger
    /// or smaller sizes than the native size of the texture. A separate filter can be
    /// set for magnification and minification.
    pub fn set_filter(&mut self, mag: TextureFilter, min: TextureFilter) {
        unsafe {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min as GLint);
        }
    }

    /// Sets the texture filter, allowing for a separate filter to be used when mipmapping
    pub fn set_mipmap_filter(&mut self, mag: TextureFilter, mipmap_mag: TextureFilter,
                             min: TextureFilter, mipmap_min: TextureFilter) {
        unsafe {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, TextureFilter::mipmap_filter(mag, mipmap_mag) as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, TextureFilter::mipmap_filter(min, mipmap_min) as GLint);
        }
    }
}

/// Represents an OpenGL texture filter. Use in OpenGL functions like ´TextureFilter::* as GLenum´
#[derive(Debug, Copy, Clone)]
pub enum TextureFilter {
    Nearest = gl::NEAREST as isize,
    Linear  = gl::LINEAR  as isize,
}
impl TextureFilter {
    /// Retrieves a OpenGL mipmap filter for mipmaping. The returned `GLenum` can
    /// be used in the same scenarios as ´TextureFilter::* as GLenum´
    fn mipmap_filter(normal: TextureFilter, mipmap: TextureFilter) -> GLenum {
        match normal {
            TextureFilter::Nearest => match mipmap {
                TextureFilter::Nearest => gl::NEAREST_MIPMAP_NEAREST,
                TextureFilter::Linear => gl::NEAREST_MIPMAP_LINEAR,
            },
            TextureFilter::Linear => match mipmap {
                TextureFilter::Nearest => gl::LINEAR_MIPMAP_NEAREST,
                TextureFilter::Linear => gl::LINEAR_MIPMAP_LINEAR,
            },
        }
    }
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, Copy, Clone)]
/// Represents a OpenGL texture format. Use in OpenGL functions like `TextureFormat::* as GLenum`
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
    /// Retrives the unsized version of the given format
    pub fn unsized_format(&self) -> GLenum {
        match *self {
            TextureFormat::RGBA_F32 | TextureFormat::RGBA_F16 | TextureFormat::RGBA_8 => gl::RGBA,
            TextureFormat::RGB_F32 | TextureFormat::RGB_F16 | TextureFormat::RGB_8 => gl::RGB,
            TextureFormat::R_F32 | TextureFormat::R_F16 | TextureFormat::R_8 => gl::RED
        }
    }
}


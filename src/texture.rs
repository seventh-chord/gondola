
use image;
use gl;
use gl::types::*;
use std::io;
use std::path::{ Path, PathBuf };

/// A wraper around a OpenGL texture object which can be modified
#[derive(Debug)]
pub struct Texture {
    source_file: Option<PathBuf>, // If this texture did not originate from a file, this will be None 
    texture: GLuint,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

/// A reference to a OpenGL texture object which can not be modified. Note that the underlying
/// data, represented by a [`Texture`](struct.Texture.html), can be modified while this object
/// is in use. If the texture this reference reffers to is dropped, this texture reference will
/// either not work or show another texture.
#[derive(Debug, Clone, Copy)]
pub struct TextureReference {
    texture: GLuint,
}

impl Texture {
    /// Attempts to load a texture from the given path
    pub fn load(path: &Path) -> io::Result<Texture> {
        let image = match image::open(path) {
            Ok(image) => image,
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };
        let image = match image {
            image::DynamicImage::ImageRgba8(image) => image,
            other => other.to_rgba() // Convert other formats to RGBA
        }; 

        let mut texture = Texture::new();
        texture.load_data(&image, image.width(), image.height(), TextureFormat::RGBA_8);
        texture.source_file = Some(PathBuf::from(path));
        Ok(texture)
    }

    /// Creates a new texture without any ascociated data
    pub fn new() -> Texture {
        let mut texture = 0;

        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        }

        Texture {
            source_file: None,
            texture: texture,
            format: TextureFormat::RGB_8,
            width: 0,
            height: 0,
        }
    }

    /// Sets the data this texture points to
    pub fn load_data(&mut self, data: &[u8], width: u32, height: u32, format: TextureFormat) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::TexImage2D(gl::TEXTURE_2D, 0, // Mipmap level
                           format as GLint, // Internal format
                           width as GLsizei, height as GLsizei, 0, // Size and border
                           format.unsized_format(), // Data format
                           gl::UNSIGNED_BYTE, data.as_ptr() as *const GLvoid);
        }

        self.width = width;
        self.height = height;
        self.format = format;
    }

    /// Reloads this texture from source, if it was originally loaded from a file 
    pub fn reload(&mut self) -> io::Result<()> {
        if self.source_file.is_some() {
            let source_file = self.source_file.clone().unwrap();
            let new = Texture::load(&source_file)?;
            // This will not be executed if loading fails
            // Replace with new texture
            unsafe { gl::DeleteTextures(1, &self.texture); }
            *self = new;
        }

        Ok(())
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

    /// Creates a reference to this texture which can not be modified
    pub fn create_reference(&self) -> TextureReference {
        TextureReference {
            texture: self.texture,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture);
        }
    }
}

impl TextureReference {
    /// Binds this texture to the given texture unit
    pub fn bind(&self, unit: u32) {
        unsafe {
            if gl::IsTexture(self.texture) != gl::TRUE {
                // Draw MISSING-NO texture
            } else {
                gl::ActiveTexture(gl::TEXTURE0 + unit as GLenum);
                gl::BindTexture(gl::TEXTURE_2D, self.texture);
            }
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


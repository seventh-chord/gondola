
//! Utilities for loading and using textures

use std::io;
use std::ptr;
use std::fmt;
use std::error;
use std::path::Path;
use std::borrow::Cow;
use std::fs::File;
use png;
use gl;
use gl::types::*;

/// A wraper around a OpenGL texture object which can be modified
#[derive(Debug)]
pub struct Texture {
    texture: GLuint,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl Texture { 
    /// Creates a texture from a raw OpenGL handle and some additional data. Intended for internal
    /// use only, use with care!
    pub fn wrap_gl_texture(texture: GLuint, format: TextureFormat, width: u32, height: u32) -> Texture {
        Texture {
            texture: texture,
            format: format,
            width: width,
            height: height,
        }
    }

    /// Creates a texture from a image file.
    pub fn from_file<P>(path: P) -> Result<Texture, TextureError> where P: AsRef<Path> {
        let mut texture = Texture::new();
        texture.load_file(path)?;
        Ok(texture)
    }

    /// Creates a new texture without any ascociated data. Use can use [`load_file`],
    /// [`load_raw_image_data`] and [`load_data`] to set the data to be used used
    /// with this texture.
    ///
    /// [`load_file`]:           struct.Texture.html#method.load_file
    /// [`load_raw_image_data`]: struct.Texture.html#method.load_raw_image_data
    /// [`load_data`]:           struct.Texture.html#method.load_data
    pub fn new() -> Texture {
        let mut texture = 0;

        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        }

        Texture {
            texture: texture,
            format: TextureFormat::RGB_8,
            width: 0,
            height: 0,
        }
    }

    /// Attempts to load data from the given image file into this texture. Note that
    /// it is usually more convenient to create a new texture directly from a file using
    /// [`from_file(path)`](struct.Texture.html#method.from_file).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gondola::texture::Texture;
    ///
    /// let mut texture = Texture::new();
    /// texture.load_file("assets/test.png").expect("Failed to load texture");
    /// ```
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), TextureError> {
        let path = path.as_ref();
        let RawImageData { info, buf } = RawImageData::from_file(path)?;
        let texture_format = match (info.color_type, info.bit_depth) {
            (png::ColorType::RGBA, png::BitDepth::Eight) => TextureFormat::RGBA_8,
            (png::ColorType::RGB, png::BitDepth::Eight)  => TextureFormat::RGB_8,
            other => {
                let message = format!(
                    "Unsuported texture format ({:?}, {:?}) in \"{}\" ({}:{})",
                    other.0, other.1,
                    path.to_string_lossy(),
                    file!(), line!()
                );

                return Err(TextureError { 
                    source: Some(path.to_string_lossy().into()),
                    error: io::Error::new(io::ErrorKind::Other, message) 
                });
            }
        };
        self.load_data(&buf, info.width, info.height, texture_format);
        Ok(())
    }

    /// Attempts to load the given raw image data into this texture. For more info see
    /// [`RawImageData`].
    ///
    /// [`RawImageData`]: struct.RawImageData.html
    pub fn load_raw_image_data(&mut self, data: RawImageData) -> Result<(), TextureError> {
        let texture_format = match (data.info.color_type, data.info.bit_depth) {
            (png::ColorType::RGBA, png::BitDepth::Eight) => TextureFormat::RGBA_8,
            (png::ColorType::RGB, png::BitDepth::Eight)  => TextureFormat::RGB_8,
            other => {
                let message = format!(
                    "Unsuported texture format ({:?}, {:?}) ({}:{})",
                    other.0, other.1, file!(), line!()
                );
                return Err(TextureError { source: None, error: io::Error::new(io::ErrorKind::Other, message) });
            }
        };
        self.load_data(&data.buf, data.info.width, data.info.height, texture_format);
        Ok(())
    }

    /// Directly loads some color data into a texture. This function does not check to ensure that
    /// the data is in the correct format, so you have to manually ensure that it is valid. This
    /// function is intended for creating small debug textures.
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

    /// Sets the data in a sub-region of this texture. The data is expected to be in the
    /// format this texture was initialized to. This texture needs to be initialized
    /// before this method can be used.
    /// Note that there is a debug assertion in place to ensure that the given region
    /// is within the bounds of this texture. If debug assertions are not enabled this
    /// function will return without taking any action.
    pub fn load_data_to_region(&mut self, data: &[u8], x: u32, y: u32, width: u32, height: u32) {
        if x + width > self.width && y + height > self.height {
            debug_assert!(false, "Invalid region passed ({}:{}) Region: (x: {}, y: {}, width: {}, height: {})",
                          file!(), line!(),
                          x, y, width, height);
            return;
        }
        unsafe {
            // OpenGL is allowed to expect rows in pixel data to be aligned
            // at powers of two. This ensures that any data will be accepted.
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);

            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::TexSubImage2D(gl::TEXTURE_2D, 0,
                              x as GLint, y as GLint,
                              width as GLsizei, height as GLsizei,
                              self.format.unsized_format(), // It is unclear whether opengl allows a different format here
                              gl::UNSIGNED_BYTE, data.as_ptr() as *const GLvoid);
        }
    }

    /// Converts this texture to a empty texture of the given size. The contents
    /// of the texture after this operation are undefined.
    pub fn initialize(&mut self, width: u32, height: u32, format: TextureFormat) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::TexImage2D(gl::TEXTURE_2D, 0, // Mipmap level
                           format as GLint, // Internal format
                           width as GLsizei, height as GLsizei, 0, // Size and border
                           format.unsized_format(), // Data format
                           gl::UNSIGNED_BYTE, ptr::null());
        }

        self.width = width;
        self.height = height;
        self.format = format;
    }

    /// Binds this texture to the given texture unit.
    pub fn bind(&self, unit: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit);
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
        }
    }

    /// Unbinds the texture at the given texture unit.
    pub fn unbind(unit: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit);
            gl::BindTexture(gl::TEXTURE_2D, 0);
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

    /// Sets the swizzle mask of this texture. The swizzle mask specifies how data stored
    /// in this texture is seen by other parts of OpenGL. This includes texture samplers
    /// in shaders. This is usefull when using textures with only one or two components
    /// per pixel.
    ///
    /// For example, given a texture with only a red component (That is, its
    /// format is `TextureFormat::R_8` or similar), a texture sampler in a shader will
    /// normaly get a value of type `(r, 0.0, 0.0, 1.0)`. By setting the swizzle mask
    /// to `(SwizzleComp::One, SwizzleComp::One, SwizzleComp::One, SwizzleComp::Red)`
    /// shaders will now see `(1.0, 1.0, 1.0, r)`.
    pub fn set_swizzle_mask(&mut self, masks: (SwizzleComp, SwizzleComp, SwizzleComp, SwizzleComp)) {
        unsafe {
            let masks = [masks.0 as GLint, masks.1 as GLint, masks.2 as GLint, masks.3 as GLint];
            gl::TexParameteriv(gl::TEXTURE_2D, gl::TEXTURE_SWIZZLE_RGBA, &masks as *const _);
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

/// Raw image data loaded from a png file. This data can then be loaded into a texture 
/// using [`Texture::load_raw_image_data`]. When loading very large textures it can be
/// beneficial to load the raw image data from the texture on a separate thread, and then
/// pass it to a texture in the main thread for performance reasons.
///
/// Note that textures must allways be created in the same thread as they are used in, because 
/// of OpenGL limitations. You can call [`RawImageData::from_file`] from anywhere, but only
/// ever create textures in the rendering tread (usually the main thread).
///
/// [`Texture::load_raw_image_data`]: struct.Texture.html#method.load_raw_image_data
/// [`RawImageData::from_file`]: struct.RawImageData.html#method.from_file
pub struct RawImageData {
    info: png::OutputInfo,
    buf: Vec<u8>,
}

impl RawImageData {
    /// Does not invoke any OpenGL functions, and can thus be called from any thread.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<RawImageData, TextureError> {
        let path = path.as_ref();

        // Open file
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => return Err(TextureError { 
                source: Some(path.to_string_lossy().into()),
                error: err 
            }),
        };

        let decoder = png::Decoder::new(file);

        RawImageData::from_decoder(decoder, path.to_string_lossy().into())
    }

    /// Can be used in conjunction with the `include_bytes!(..)` in std.
    pub fn from_bytes(bytes: &[u8], source: &str) -> Result<RawImageData, TextureError> {
        RawImageData::from_decoder(png::Decoder::new(bytes), source.into())
    }

    fn from_decoder<R: io::Read>(
        decoder: png::Decoder<R>,
        source: Cow<str>,
    ) -> Result<RawImageData, TextureError> 
    {
        let (info, mut reader) = match decoder.read_info() {
            Ok(result) => result,
            Err(err) => return Err(TextureError { 
                source: Some(source.into()),
                error: err.into() 
            }),
        };

        // Read data into buffer (This is what makes texture loading slow)
        let mut buf = vec![0; info.buffer_size()];
        match reader.next_frame(&mut buf) {
            Ok(()) => {},
            Err(err) => return Err(TextureError {
                source: Some(source.into()),
                error: err.into() 
            }),
        };

        Ok(RawImageData {
            info: info,
            buf: buf,
        })
    }
}

/// Represents an OpenGL texture filter.
#[repr(u32)] // GLenum is u32
#[derive(Debug, Copy, Clone)]
pub enum TextureFilter {
    Nearest = gl::NEAREST,
    Linear  = gl::LINEAR
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

/// Represents a OpenGL texture format.
#[repr(u32)] // GLenum is u32
#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum TextureFormat {
    RGBA_F32 = gl::RGBA32F,
    RGBA_F16 = gl::RGBA16F,
    RGB_F32  = gl::RGB32F,
    RGB_F16  = gl::RGB16F,
    R_F32    = gl::R32F,
    R_F16    = gl::R16F,

    RGBA_8   = gl::RGBA8,
    RGB_8    = gl::RGB8,
    R_8      = gl::R8,
}
impl TextureFormat {
    /// Retrieves the unsized version of the given format
    pub fn unsized_format(&self) -> GLenum {
        match *self {
            TextureFormat::RGBA_F32 | TextureFormat::RGBA_F16 | TextureFormat::RGBA_8 => gl::RGBA,
            TextureFormat::RGB_F32 | TextureFormat::RGB_F16 | TextureFormat::RGB_8 => gl::RGB,
            TextureFormat::R_F32 | TextureFormat::R_F16 | TextureFormat::R_8 => gl::RED,
        }
    }

    /// The OpenGL primitive associated with this color format.
    pub fn gl_primitive_enum(&self) -> GLenum {
        match *self {
            TextureFormat::RGBA_F32 | TextureFormat::RGB_F32 | TextureFormat::R_F32 => gl::FLOAT,
            TextureFormat::RGBA_F16 | TextureFormat::RGB_F16 | TextureFormat::R_F16 => gl::FLOAT,
            TextureFormat::RGBA_8 | TextureFormat::RGB_8 | TextureFormat::R_8 => gl::UNSIGNED_BYTE,
        }
    }

    /// The name of the OpenGL primitive associated with this color format.
    pub fn gl_primitive_enum_name(&self) -> &'static str {
        match *self {
            TextureFormat::RGBA_F32 | TextureFormat::RGB_F32 | TextureFormat::R_F32 => "GLfloat",
            TextureFormat::RGBA_F16 | TextureFormat::RGB_F16 | TextureFormat::R_F16 => "GLfloat",
            TextureFormat::RGBA_8 | TextureFormat::RGB_8 | TextureFormat::R_8 => "GLbyte",
        }
    }

    /// The number of components this color format has. For example, `RGB_8` has 3 components.
    pub fn components(&self) -> usize {
        match *self {
            TextureFormat::RGBA_F32 | TextureFormat::RGBA_F16 | TextureFormat::RGBA_8 => 4,
            TextureFormat::RGB_F32 | TextureFormat::RGB_F16 | TextureFormat::RGB_8 => 3,
            TextureFormat::R_F32 | TextureFormat::R_F16 | TextureFormat::R_8 => 1,
        }
    }
}

/// Components that a texture can be mapped to through swizzling. See
/// [`set_swizzle_mask`](struct.Texture.html#method.set_swizzle_mask)
/// for more info.
#[repr(u32)] // GLenum is u32
#[derive(Debug, Copy, Clone)]
pub enum SwizzleComp {
    Red     = gl::RED,
    Green   = gl::GREEN,
    Blue    = gl::BLUE,
    Alpha   = gl::ALPHA,
    One     = gl::ONE,
    Zero    = gl::ZERO,
}

/// A error which can occur during texture loading and creation.
#[derive(Debug)]
pub struct TextureError {
    source: Option<String>,
    error: io::Error,
}

impl error::Error for TextureError {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&error::Error> {
        self.error.cause()
    }
}

impl fmt::Display for TextureError {
    fn fmt(&self, mut f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref source) = self.source {
            write!(f, "For texture \"{}\": ", source)?;
        }

        self.error.fmt(&mut f)?;
        Ok(())
    }
}

impl From<TextureError> for io::Error {
    fn from(err: TextureError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

/// Includes the binary data from a texture file needed to load a texture in the binary. The
/// texture is ATM still decoded at runtime. 
#[macro_export]
macro_rules! include_texture { ($SOURCE:expr) => {{
    let _: &str = $SOURCE; // Type-checking

    let mut texture = $crate::texture::Texture::new();

    let bytes = include_bytes!($SOURCE);
    let data = match $crate::texture::RawImageData::from_bytes(bytes, $SOURCE) {
        Ok(d) => d,
        Err(err) => panic!("Could not decode {}: {}", $SOURCE, err),
    };

    match texture.load_raw_image_data(data) {
        Ok(()) => {},
        Err(err) => panic!("Could not decode {}: {}", $SOURCE, err),
    }

    texture
}}; }


//! Framebuffers are used to draw to off-screen render targets

use gl;
use std::fmt;
use std::error;
use gl::types::*;

use color::Color;
use texture::TextureFormat;
use buffer::{VertexData, GlPrimitive};

use cable_math::Vec2;

/// Set to 8, which 97% of all cards support, acording to the [wildfiregames report][1]
/// [1]: http://feedback.wildfiregames.com/report/opengl/feature/GL_MAX_COLOR_ATTACHMENTS_EXT
pub const MAX_COLOR_ATTACHMENTS: usize = 8;

/// Utility to specify the format of a framebuffer before building it. If you expect to rebuild a
/// framebuffer occasionally (e.g. when the game window is resized) it could be beneficial to store
/// this struct alongside the framebuffer itself.
#[derive(Debug, Clone, Default)]
pub struct FramebufferProperties {
    /// Size in pixels
    pub size:  Vec2<u32>,
    /// The amount of multisampling to apply. If this is greater than the value returned by 
    /// `framebuffer::max_sample_level` building a framebuffer with these properties will panic.
    pub multisample: Option<usize>,
    /// The color formats in which color data is stored internally. The OpenGL spec states that at
    /// least 8 attachments will be supported, and in practice no card supports more than this.
    pub color_formats: [Option<TextureFormat>; MAX_COLOR_ATTACHMENTS],
    /// If `true` a depthbuffer will be added to framebuffers
    pub depth_buffer: bool,
}

impl FramebufferProperties {
    pub fn new(size: Vec2<u32>) -> FramebufferProperties {
        FramebufferProperties {
            size,
            multisample: None,
            color_formats: [Some(TextureFormat::RGB_8), None, None, None, None, None, None, None],
            depth_buffer: false,
        }
    }

    /// Creates a new framebuffer with these properties
    pub fn build(&self) -> Result<Framebuffer, FramebufferError> {
        Framebuffer::new(&self)
    }
}

/// A OpenGL framebuffer that is ready to be used. Framebuffers are constructed from
/// [`FramebufferProperties`](struct.FramebufferProperties.html).
pub struct Framebuffer {
    framebuffer: GLuint,
    color_attachments: [Option<ColorAttachmentData>; MAX_COLOR_ATTACHMENTS],
    depth_buffer: Option<GLuint>,
    pub size: Vec2<u32>,
}

// This struct must NOT be Clone or Copy
pub struct ColorAttachmentData {
    handle: GLuint,
    format: TextureFormat,
    multisampled: bool,
}

impl Framebuffer {
    fn new(properties: &FramebufferProperties) -> Result<Framebuffer, FramebufferError> {
        // Validity checks
        if let Some(samples) = properties.multisample {
            let max = max_samples();
            if samples > max {
                panic!(
                    "Tried creating a framebuffer with multisampling level {}, \
                     but {} is the max level supported",
                    samples, max
                );
            }
        }

        // Actually build the framebuffer
        let mut framebuffer: GLuint = 0;
        let mut color_attachments: [Option<ColorAttachmentData>; MAX_COLOR_ATTACHMENTS] = Default::default();
        let mut depth_buffer: Option<GLuint> = None;

        let mut error: Option<FramebufferError> = None;

        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);

            let texture_target = if properties.multisample.is_none() { gl::TEXTURE_2D } else { gl::TEXTURE_2D_MULTISAMPLE };

            // Add draw buffers
            let mut draw_buffers: [GLenum; MAX_COLOR_ATTACHMENTS] = Default::default();
            for i in 0..MAX_COLOR_ATTACHMENTS {
                // Add a color attachment
                if let Some(format) = properties.color_formats[i] {
                    let attachment = gl::COLOR_ATTACHMENT0 + (i as GLenum);
                    draw_buffers[i] = attachment;

                    let mut texture = 0;
                    gl::GenTextures(1, &mut texture);
                    gl::BindTexture(texture_target, texture);
                    if let Some(level) = properties.multisample {
                        gl::TexImage2DMultisample(
                            texture_target,
                            level as GLsizei,
                            format as GLuint,
                            properties.size.x as GLint, properties.size.y as GLint,
                            true as GLboolean, // Fixed sample locations
                        );
                    } else {
                        gl::TexImage2D(
                            texture_target,
                            0, // Level
                            format as GLint,
                            properties.size.x as GLint, properties.size.y as GLint, 0, //Size and border
                            format.unsized_format(), format.gl_primitive_enum(), 
                            ::std::ptr::null()
                        ); // Data for texture
                        gl::TexParameteri(texture_target, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
                        gl::TexParameteri(texture_target, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
                        gl::TexParameteri(texture_target, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
                        gl::TexParameteri(texture_target, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as GLint);
                        gl::TexParameteri(texture_target, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as GLint);
                    }

                    gl::FramebufferTexture(gl::FRAMEBUFFER, attachment, texture, 0);
                    color_attachments[i] = Some(ColorAttachmentData {
                        handle: texture,
                        format: format,
                        multisampled: properties.multisample.is_some(),
                    });
                } else {
                    draw_buffers[i] = gl::NONE;
                }
                
            }

            gl::DrawBuffers(MAX_COLOR_ATTACHMENTS as GLsizei, draw_buffers.as_ptr());

            // Add depth buffer
            if properties.depth_buffer {
                let mut depth_buffer_handle = 0;
                gl::GenRenderbuffers(1, &mut depth_buffer_handle);
                gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer_handle);
                if let Some(level) = properties.multisample {
                    gl::RenderbufferStorageMultisample(
                        gl::RENDERBUFFER,
                        level as GLsizei,
                        gl::DEPTH_COMPONENT, 
                        properties.size.x as GLint,
                        properties.size.y as GLint
                    );
                } else {
                    gl::RenderbufferStorage(
                        gl::RENDERBUFFER,
                        gl::DEPTH_COMPONENT, 
                        properties.size.x as GLint,
                        properties.size.y as GLint
                    );
                }
                gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::RENDERBUFFER, depth_buffer_handle);
                depth_buffer = Some(depth_buffer_handle);
            }

            // Check if framebuffer was sucessfully constructed
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                // Delete OpenGL data (Color attachments are automatically deleted)
                gl::DeleteFramebuffers(1, &framebuffer);
                if let Some(depth_buffer) = depth_buffer {
                    gl::DeleteRenderbuffers(1, &depth_buffer);
                }
                error = Some(From::from(status));
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        if let Some(error) = error {
            return Err(error);
        } else {
            return Ok(
                Framebuffer {
                    framebuffer: framebuffer,
                    color_attachments: color_attachments,
                    depth_buffer: depth_buffer,
                    size: properties.size,
                }
            );
        }
    }

    /// Binds this framebuffer. Subsequent draw operations will modify this framebuffer
    /// rather than the backbuffer. Note that you probably want to modify the viewport
    /// to fit this framebuffers size.
    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);
        }
    }
    
    /// Binds framebuffer 0, resulting in draw operations drawing to the backbuffer.
    pub fn unbind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    /// Moves the contents of this framebuffer to the given framebuffer, resolving multisampling
    /// if present. Note that this also unbinds this framebuffer
    pub fn blit_to_framebuffer(&self, other: &Framebuffer, buffers: Blit) {
        self.blit_indexed(other.framebuffer, other.size, buffers);
    }

    /// Moves the contents of this framebuffer to the backbuffer, resolving multisampling
    /// if present. Note that this also unbinds this framebuffer. This will only partially
    /// cover the backbuffer if this framebuffer is smaller than the backbuffer. To upscale
    /// a framebuffer while blitting, use [`blit_with_size`](struct.Framebuffer.html#method.blit_with_size).
    pub fn blit(&self, buffers: Blit) {
        self.blit_indexed(0, self.size, buffers);
    }

    /// Moves the contents of this framebuffer to the backbuffer, resolving multisampling
    /// if present. Note that this also unbinds this framebuffer. This allows setting
    /// the size to which this framebuffer should be scaled while blitting. This should
    /// be used if the framebuffer is larger or smaller than the backbuffer.
    pub fn blit_with_size(&self, size: Vec2<u32>, buffers: Blit) {
        self.blit_indexed(0, size, buffers);
    }

    fn blit_indexed(&self, target: GLuint, dst_size: Vec2<u32>, buffers: Blit) {
        let mut gl_flag = 0;
        if buffers.color   { gl_flag |= gl::COLOR_BUFFER_BIT }
        if buffers.depth   { gl_flag |= gl::DEPTH_BUFFER_BIT }
        if buffers.stencil { gl_flag |= gl::STENCIL_BUFFER_BIT }

        unsafe {
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, target);
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.framebuffer);
            gl::BlitFramebuffer(0, 0, self.size.x as i32, self.size.y as i32,
                                0, 0, dst_size.x as i32, dst_size.y as i32,
                                gl_flag, gl::NEAREST);
        }
        self.unbind();
    }

    /// Retrieves the color attachment at the given index. There will be a color attachment at each
    /// index for which a `color_format`  was set in the [framebuffers properties][1] from which this
    /// framebuffer was built. If there is no color attachment at the given index, or the index is
    /// greater than [`MAX_COLOR_ATTACHMENTS`][2] this returns `None`.
    ///
    /// Color attachments can be bound to either `GL_TEXTURE_2D` or `GL_TEXTURE_2D_MULTISAMPLE`
    /// depending on whether multisampling is enabled for this framebuffer. See
    /// [`ColorAttachmentData`] for more info.
    /// 
    /// [1]: struct.FramebufferProperties.html
    /// [2]: constant.MAX_COLOR_ATTACHMENTS.html
    /// [`ColorAttachmentData`]: struct.ColorAttachmentData.html
    pub fn get_color_attachment(&self, index: usize) -> Option<&ColorAttachmentData> {
        if index < self.color_attachments.len() {
            if let Some(ref color_attachment) = self.color_attachments[index] {
                return Some(color_attachment);
            }
        } 
        None
    }

    /// Clears the color attachment at the given index to the given color. This method panics if
    /// the index is not that of a valid color attachment.
    pub fn clear_color_attachment(&self, index: usize, color: Color) {
        if index > MAX_COLOR_ATTACHMENTS || self.color_attachments[index].is_none() {
            panic!("Could not clear framebuffer color attachment. {} is not a valid color attachment index", index);
        }

        unsafe {
            use std::mem;
            gl::ClearBufferfv(gl::COLOR, index as GLint, mem::transmute(&color));
        }
    }

    /// Retrieves the pixels from the given region in the given color attachment. Returns all
    /// pixels in row-major order. Because a framebuffers attachments types are not strongly typed
    /// it is critical that `T` is a type which has the same format as the color attachment.
    ///
    /// # Panics
    ///
    ///  * If the region is outside of the bounds of this framebuffer. 
    ///  * If the index does not point to a valid color attachment.
    ///  * If T has a different number of primitives than the given color attachment.
    ///  * If T has a different primitive type than the given color attachment.
    pub fn get_pixel_data<T>(&self, index: usize, pos: Vec2<u32>, size: Vec2<u32>) -> Vec<T> 
        where T: VertexData,
    {
        let mut data = Vec::<T>::with_capacity((size.x * size.y) as usize);

        if index > MAX_COLOR_ATTACHMENTS && self.color_attachments[index].is_none() {
            panic!("Invalid call to get_pixel_data. {} is not a valid color attachment.", index);
        }
        let format = match self.color_attachments[index] {
            Some(ref attachment) => attachment.format,
            None => unreachable!(), // We check if texture is None above
        };

        if T::primitives() != format.components() {
            panic!(
                "Invalid call to get_pixel_data. T has a different number of primitives than {:?}.", 
                format,
            );
        }

        if T::Primitive::gl_enum() != format.gl_primitive_enum() {
            panic!(
                "Invalid call to get_pixel_data. T has a different primitive type than color \
                attachment {}. ({} vs {})",
                index, T::Primitive::gl_name(), format.gl_primitive_enum_name(),
            );
        }

        if pos.x + size.x > self.size.x || pos.y + size.y > self.size.y {
            panic!(
                "Invalid call to get_pixel_data, The rectangle (pos: {}, size: {}) is outside of \
                the region of the framebuffer (framebuffer size: {}).",
                pos, size, self.size,
            );
        }

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);
            gl::ReadBuffer(gl::COLOR_ATTACHMENT0 + index as u32);
            gl::ReadPixels(
                pos.x as GLint, pos.y as GLint, 
                size.x as GLsizei, size.y as GLsizei,
                format.unsized_format(),
                format.gl_primitive_enum(),
                data.as_ptr() as *mut _
            );
            data.set_len((size.x * size.y) as usize);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        data
    }
}

// The max value that `FramebufferProperties::multisample` may have
pub fn max_samples() -> usize {
    let mut result = 0;
    unsafe {
        gl::GetIntegerv(gl::MAX_SAMPLES, &mut result);
    }
    result as usize
}

impl ColorAttachmentData {
    /// Binds this color attachment to the given texture unit. If this color attachment belongs to
    /// a multisampled framebuffer the texture is bound to `GL_TEXTURE_2D_MULTISAMPLE`. Otherwise,
    /// the texture is bound to `GL_TEXTURE_2D`.
    pub fn bind(&self, unit: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit);
            let target = if self.multisampled { gl::TEXTURE_2D_MULTISAMPLE } else { gl::TEXTURE_2D };
            gl::BindTexture(target, self.handle);
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.framebuffer);
            if let Some(depth_buffer) = self.depth_buffer {
                gl::DeleteRenderbuffers(1, &depth_buffer);
            }
            // Color attachments are managed by the `ColorAttachmentData` struct, and are automatically deleted
        }
    }
}

impl Drop for ColorAttachmentData {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.handle);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Blit {
    pub color: bool,
    pub depth: bool,
    pub stencil: bool,
}

impl Default for Blit {
    fn default() -> Blit {
        Blit {
            color: true,
            depth: false,
            stencil: false,
        }
    }
}

/// A error which can occure while constructing a framebuffer in OpenGL. The variants of this enum
/// corespond to those `gl::FRAMEBUFFER_*` constants which are errors.
#[derive(Debug, Clone)]
pub enum FramebufferError {
    Undefined,
    IncompleteAttachment,
    IncompleteMissingAttachment,
    IncompleteDrawBuffer,
    IncompleteReadBuffer,
    Unsuported,
    IncompleteMultisample,
    IncompleteLayerTargets,
    UnkownError(GLenum),
}

impl From<GLenum> for FramebufferError {
    fn from(err: GLenum) -> FramebufferError {
        match err {
            gl::FRAMEBUFFER_UNDEFINED                     => FramebufferError::Undefined,
            gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT         => FramebufferError::IncompleteAttachment,
            gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => FramebufferError::IncompleteMissingAttachment,
            gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER        => FramebufferError::IncompleteDrawBuffer,
            gl::FRAMEBUFFER_INCOMPLETE_READ_BUFFER        => FramebufferError::IncompleteReadBuffer,
            gl::FRAMEBUFFER_UNSUPPORTED                   => FramebufferError::Unsuported,
            gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE        => FramebufferError::IncompleteMultisample,
            gl::FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS      => FramebufferError::IncompleteLayerTargets,
            _                                             => FramebufferError::UnkownError(err),
        }
    }
}

impl error::Error for FramebufferError {
    fn description(&self) -> &str {
        match *self {
            FramebufferError::Undefined                     => "Framebuffer error: Undefined framebuffer",
            FramebufferError::IncompleteAttachment          => "Framebuffer error: Incompelete attachment",
            FramebufferError::IncompleteMissingAttachment   => "Framebuffer error: Incompelete missing attachment",
            FramebufferError::IncompleteDrawBuffer          => "Framebuffer error: Incomplete draw buffer",
            FramebufferError::IncompleteReadBuffer          => "Framebuffer error: Incomplete read buffer",
            FramebufferError::Unsuported                    => "Framebuffer error: Unsuported",
            FramebufferError::IncompleteMultisample         => "Framebuffer error: Incomplete multisample",
            FramebufferError::IncompleteLayerTargets        => "Framebuffer error: Incomplete layer targets",
            FramebufferError::UnkownError(_)                => "Framebuffer error: Unkown error code",
        }
    }
}

impl fmt::Display for FramebufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FramebufferError::Undefined                     => write!(f, "Framebuffer error: Undefined framebuffer"),
            FramebufferError::IncompleteAttachment          => write!(f, "Framebuffer error: Incompelete attachment"),
            FramebufferError::IncompleteMissingAttachment   => write!(f, "Framebuffer error: Incompelete missing attachment"),
            FramebufferError::IncompleteDrawBuffer          => write!(f, "Framebuffer error: Incomplete draw buffer"),
            FramebufferError::IncompleteReadBuffer          => write!(f, "Framebuffer error: Incomplete read buffer"),
            FramebufferError::Unsuported                    => write!(f, "Framebuffer error: Unsuported"),
            FramebufferError::IncompleteMultisample         => write!(f, "Framebuffer error: Incomplete multisample"),
            FramebufferError::IncompleteLayerTargets        => write!(f, "Framebuffer error: Incomplete layer targets"),
            FramebufferError::UnkownError(code)             => write!(f, "Framebuffer error: Unkown error code: 0x{:x}", code),
        }
    }
}


//! Framebuffers are used to draw to off-screen render targets

use gl;
use std::fmt;
use std::error;
use gl::types::*;
use texture::TextureFormat;

/// Utility to specify the format of a framebuffer before building it.
pub struct FramebufferProperties {
    /// Size in pixels
    pub width: u32,
    /// Size in pixels
    pub height: u32,
    /// The amount of multisampling to apply
    pub multisample: Option<usize>,
    /// The format in which color data is stored internally
    pub internal_format: TextureFormat,
    /// If `true` a depthbuffer will be constructed for framebuffers
    pub depth_buffer: bool,
}

impl FramebufferProperties {
    pub fn new(width: u32, height: u32) -> FramebufferProperties {
        FramebufferProperties {
            width: width,
            height: height,
            multisample: None,
            internal_format: TextureFormat::RGB_8,
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
    texture: GLuint,
    depth_buffer: Option<GLuint>,
    pub width: u32,
    pub height: u32
}

impl Framebuffer {
    fn new(properties: &FramebufferProperties) -> Result<Framebuffer, FramebufferError> {
        let mut framebuffer: GLuint = 0;
        let mut texture: GLuint = 0;
        let mut depth_buffer: Option<GLuint> = None;

        let mut error: Option<FramebufferError> = None;

        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);

            let texture_target = if properties.multisample.is_none() { gl::TEXTURE_2D } else { gl::TEXTURE_2D_MULTISAMPLE };

            gl::GenTextures(1, &mut texture);
            gl::BindTexture(texture_target, texture);
            ::util::graphics::print_errors();
            if let Some(level) = properties.multisample {
                gl::TexImage2DMultisample(texture_target,
                                          level as GLsizei,
                                          properties.internal_format as GLuint,
                                          properties.width as GLint, properties.height as GLint, //Size
                                          true as GLboolean); // Fixed sample locations
            } else {
                gl::TexImage2D(texture_target,
                               0, // Level
                               properties.internal_format as GLint,
                               properties.width as GLint, properties.height as GLint, 0, //Size and border
                               gl::RGBA, gl::UNSIGNED_BYTE, ::std::ptr::null()); // Data for texture
                gl::TexParameteri(texture_target, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
                gl::TexParameteri(texture_target, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
                gl::TexParameteri(texture_target, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
                gl::TexParameteri(texture_target, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
                gl::TexParameteri(texture_target, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
            }
            

            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, texture, 0);
            gl::DrawBuffers(1, &gl::COLOR_ATTACHMENT0);

            if properties.depth_buffer {
                let mut depth_buffer_handle = 0;
                gl::GenRenderbuffers(1, &mut depth_buffer_handle);
                gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer_handle);
                if let Some(level) = properties.multisample {
                    gl::RenderbufferStorageMultisample(gl::RENDERBUFFER, level as GLsizei, gl::DEPTH_COMPONENT16, 
                                                       properties.width as GLint, properties.height as GLint);
                } else {
                    gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT16,
                                            properties.width as GLint, properties.height as GLint);
                }
                gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::RENDERBUFFER, depth_buffer_handle);
                depth_buffer = Some(depth_buffer_handle);
            }

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                gl::DeleteFramebuffers(1, &framebuffer);
                gl::DeleteTextures(1, &texture);
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
                    texture: texture,
                    depth_buffer: depth_buffer,
                    width: properties.width,
                    height: properties.height,
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
    pub fn blit_to_framebuffer(&self, other: &Framebuffer) {
        self.blit_indexed(other.framebuffer, other.width, other.height);
    }

    /// Moves the contents of this framebuffer to the backbuffer, resolving multisampling
    /// if present. Note that this also unbinds this framebuffer. This will only partially
    /// cover the backbuffer if this framebuffer is smaller than the backbuffer. To upscale
    /// a framebuffer while blitting, use [`blit_with_size`](struct.Framebuffer.html#method.blit_with_size).
    pub fn blit(&self) {
        self.blit_indexed(0, self.width, self.height);
    }

    /// Moves the contents of this framebuffer to the backbuffer, resolving multisampling
    /// if present. Note that this also unbinds this framebuffer. This allows setting
    /// the size to which this framebuffer should be scaled while blitting. This should
    /// be used if the framebuffer is larger or smaller than the backbuffer.
    pub fn blit_with_size(&self, width: u32, height: u32) {
        self.blit_indexed(0, width, height);
    }

    fn blit_indexed(&self, target: GLuint, dst_width: u32, dst_height: u32) {
        unsafe {
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, target);
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.framebuffer);
            gl::BlitFramebuffer(
                0, 0, self.width as i32, self.height as i32,
                0, 0, dst_width as i32, dst_height as i32,
                gl::COLOR_BUFFER_BIT,
                gl::NEAREST
            );
        }
        self.unbind();
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.framebuffer);
            gl::DeleteTextures(1, &self.texture);
            if let Some(depth_buffer) = self.depth_buffer {
                gl::DeleteRenderbuffers(1, &depth_buffer);
            }
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

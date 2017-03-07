
//! Framebuffers are used to draw to off-screen render targets

use gl;
use std;
use std::io;
use gl::types::*;
use texture::TextureFormat;

/// Utility to specify the format of a framebuffer before building it.
pub struct FramebufferProperties {
    /// Size in pixels
    pub width: u32,
    /// Size in pixels
    pub height: u32,
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
            internal_format: TextureFormat::RGB_8,
            depth_buffer: false,
        }
    }

    /// Creates a new framebuffer with these properties
    pub fn build(&self) -> io::Result<Framebuffer> {
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
    fn new(properties: &FramebufferProperties) -> io::Result<Framebuffer> {
        let mut framebuffer: GLuint = 0;
        let mut texture: GLuint = 0;
        let mut depth_buffer: Option<GLuint> = None;

        let mut error: Option<String> = None;

        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);

            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0, // Level
                properties.internal_format as GLint,
                properties.width as GLint, properties.height as GLint, 0, //Size and border
                gl::RGBA, gl::UNSIGNED_BYTE, std::ptr::null() // Data for texture
            );

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);

            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, texture, 0);
            gl::DrawBuffers(1, &gl::COLOR_ATTACHMENT0);

            if properties.depth_buffer {
                let mut depth_buffer_handle = 0;
                gl::GenRenderbuffers(1, &mut depth_buffer_handle);
                gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer_handle);
                gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT16,
                                        properties.width as GLint, properties.height as GLint);
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
                error = Some(format!("Framebuffer error: {}", get_status_message(status)));
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        if let Some(error) = error {
            return Err(io::Error::new(io::ErrorKind::Other, error));
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

fn get_status_message(message: GLenum) -> String {
    String::from(match message {
        gl::FRAMEBUFFER_UNDEFINED                     => "GL_FRAMEBUFFER_UNDEFINED",
        gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT         => "GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT",
        gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => "GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT",
        gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER        => "GL_FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER",
        gl::FRAMEBUFFER_INCOMPLETE_READ_BUFFER        => "GL_FRAMEBUFFER_INCOMPLETE_READ_BUFFER",
        gl::FRAMEBUFFER_UNSUPPORTED                   => "GL_FRAMEBUFFER_UNSUPPORTED",
        gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE        => "GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE",
        gl::FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS      => "GL_FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS",
        _ => return format!("Unkown error ({})", message)
    })
}


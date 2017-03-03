
//! Framebuffers are used to draw to off-screen render targets

use gl;
use std;
use gl::types::*;
use texture::TextureFormat;

/// Utility to specify the format of a framebuffer before building it.
pub struct FramebufferProperties {
    pub width: u32,
    pub height: u32,
    pub internal_format: TextureFormat,
}

impl FramebufferProperties {
    pub fn new(width: u32, height: u32) -> FramebufferProperties {
        FramebufferProperties {
            width: width,
            height: height,
            internal_format: TextureFormat::RGB_8
        }
    }

    /// Creates a new framebuffer with these properties
    pub fn build(&self) -> Result<Framebuffer, String> {
        Framebuffer::new(&self)
    }
}

pub struct Framebuffer {
    framebuffer: GLuint,
    texture: GLuint,
    pub width: u32,
    pub height: u32,
}

impl Framebuffer {
    fn new(properties: &FramebufferProperties) -> Result<Framebuffer, String> {
        let mut framebuffer: GLuint = 0;
        let mut texture: GLuint = 0;

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

            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                gl::DeleteFramebuffers(1, &framebuffer);
                gl::DeleteTextures(1, &texture);
                error = Some(format!("Framebuffer error: {}", get_status_message(status)));
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
            gl::DeleteFramebuffers(1, &mut self.framebuffer);
            gl::DeleteTextures(1, &mut self.texture);
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


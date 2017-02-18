
use gl;
use gl::types::*;

/// Sets the opengl viewport
pub fn viewport(x: u32, y: u32, width: u32, height: u32) {
    unsafe {
        gl::Viewport(x as GLint, y as GLint, width as GLsizei, height as GLsizei);
    }
}

/// Prints all OpenGL errors
pub fn print_errors() {
    unsafe {
        while let Some(error) = get_error_message(gl::GetError()) {
            println!("OpenGL error: {}", error);
        }
    }
}

/// Retrieves the strign asscociated with the given OpenGL error. Returns None
/// if there if no error occured.
pub fn get_error_message(error: GLenum) -> Option<String> {
    let value = match error {
        gl::INVALID_VALUE                   => "Invalid value",
        gl::INVALID_ENUM                    => "Invalid enum",
        gl::INVALID_OPERATION               => "Invalid operation",
        gl::INVALID_FRAMEBUFFER_OPERATION   => "Invalid framebuffer operation",
        gl::OUT_OF_MEMORY                   => "Out of memory",

        gl::NO_ERROR                        => return None,
        _                                   => return Some(format!("Invalid error code: {:x}", error)),
    };
    Some(String::from(value))
}

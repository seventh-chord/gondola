
use gl;
use gl::types::*;
use std::ptr;
use std::str;
use std::ffi::CString;
use vertex_buffer::Vertex;

pub struct Shader {
    program: GLuint,
    vertex_shader: GLuint,
    geometry_shader: Option<GLuint>,
    fragment_shader: Option<GLuint>
}

impl Shader {
    pub fn with_vertex<T>(vertex_source: &str,
                          geometry_source: Option<&str>,
                          fragment_source: Option<&str>)
                          -> Result<Shader, String>
                          where T: Vertex {
        let input_decl = <T as Vertex>::gen_shader_input_decl();
        let vertex_source = &prepend_code(vertex_source, &input_decl);
        Shader::new(vertex_source , geometry_source, fragment_source)
    }

    /// Constructs a glsl shader from source. Note that the geometry and fragment shaders are
    /// optional
    pub fn new(vertex_source: &str,
               geometry_source: Option<&str>,
               fragment_source: Option<&str>)
               -> Result<Shader, String> {
        let program;
        let vertex_shader = 0;
        let fragment_shader;
        let geometry_shader;

        unsafe {
            program = gl::CreateProgram();

            let vertex_shader = compile(vertex_source, gl::VERTEX_SHADER)?;
            gl::AttachShader(program, vertex_shader);

            geometry_shader = {
                if let Some(geometry_source) = geometry_source {
                    let geometry_shader = compile(geometry_source, gl::GEOMETRY_SHADER)?;
                    gl::AttachShader(program, geometry_shader);
                    Some(geometry_shader)
                } else {
                    None
                }
            };

            fragment_shader = {
                if let Some(fragment_source) = fragment_source {
                    let fragment_shader = compile(fragment_source, gl::FRAGMENT_SHADER)?;
                    gl::AttachShader(program, fragment_shader);
                    Some(fragment_shader)
                } else {
                    None
                }
            };

            gl::LinkProgram(program);

            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status != (gl::TRUE as GLint) {
                let mut log_len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len);

                let mut buffer = Vec::with_capacity(log_len as usize);
                buffer.set_len((log_len as usize) - 1); // Skip null terminator
                gl::GetProgramInfoLog(program, log_len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);

                gl::DeleteProgram(program);
                gl::DeleteShader(vertex_shader);
                if let Some(geometry_shader) = geometry_shader {
                    gl::DeleteShader(geometry_shader);
                }
                if let Some(fragment_shader) = fragment_shader {
                    gl::DeleteShader(fragment_shader);
                }

                let message = str::from_utf8(&buffer).ok().expect("Shader log is not valid utf8").to_string();
                return Err(message);
            }
        }

        Ok(Shader {
            program: program,
            vertex_shader: vertex_shader,
            geometry_shader: geometry_shader,
            fragment_shader: fragment_shader,
        })
    }

    /// Binds this shader, replacing the previously bound shader. Subsequent draw calls
    /// will use this shader. Note that there is no method provided to unbind a shader,
    /// as it should never be necesarry.
    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.program);
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            gl::DeleteShader(self.vertex_shader);
            if let Some(geometry_shader) = self.geometry_shader {
                gl::DeleteShader(geometry_shader);
            }
            if let Some(fragment_shader) = self.fragment_shader {
                gl::DeleteShader(fragment_shader);
            }
        }
    }
}

/// Prepends the given section of code to the beginning of the given piece of
/// shader source. Note that code is inserted after the `#version ...`
/// preprocessor, if present
pub fn prepend_code(source: &str, code: &str) -> String {
    let insert_index =
        if let Some(preprocessor_index) = source.find("#version") {
            if let Some(newline_index) = source[preprocessor_index..].find('\n') {
                newline_index + preprocessor_index
            } else {
                // We might want to warn the user in this case. A shader with a
                // #version preprocessor but no newline will (I think) never
                // be valid, unless the code inserted here makes it valid
                source.len() 
            }
        } else {
            0
        };

    let mut result = String::with_capacity(source.len() + code.len());

    result.push_str(&source[0..insert_index]);
    result.push_str(code);
    if !source.is_empty() && insert_index < source.len() - 1 {
        result.push_str(&source[insert_index+1..]);
    }

    result
}

fn compile(source: &str, shader_type: GLenum) -> Result<GLuint, String> {
    unsafe {
        let shader = gl::CreateShader(shader_type);

        let c_str = CString::new(source.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        if status != (gl::TRUE as GLint) {
            let mut log_len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len);

            let mut buffer = Vec::with_capacity(log_len as usize);
            buffer.set_len((log_len as usize) - 1); // Skip null terminator
            gl::GetShaderInfoLog(shader, log_len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);

            gl::DeleteShader(shader);

            let message = str::from_utf8(&buffer).ok().expect("Shader log is not valid utf8").to_string();
            return Err(message);
        } else {
            return Ok(shader);
        }
    }
}

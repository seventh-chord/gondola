
use gl;
use gl::types::*;
use std::ptr;
use std::str;
use std::fs::File;
use std::path::Path;
use std::io;
use std::io::{BufRead, BufReader};
use std::ffi::CString;
use buffer::Vertex;
use cable_math::{Mat4, Vec2, Vec3, Vec4};

/// A shader that has not yet been fully compiled
#[derive(Debug)]
pub struct ShaderPrototype {
    vert_src: String,
    frag_src: String,
    geom_src: String,
}

impl ShaderPrototype {
    /// Loads a shader from a file. The file should contain all the shader stages, with
    /// each shader stage prepended by `-- {name}`, where name is one of `VERT`, `FRAG`
    /// or `GEOM`.
    /// # Example file
    /// ```glsl
    /// -- VERT
    /// in vec2 position;
    /// void main() {
    ///     gl_Position = vec4(position, 0.0, 1.0);
    /// }
    /// -- FRAG
    /// out vec4 color;
    /// void main() {
    ///     color = vec4(1.0, 0.0, 0.0, 1.0); // Draw in red
    /// }
    /// ```
    pub fn from_file<P>(path: P) -> io::Result<ShaderPrototype> where P: AsRef<Path> {
        let mut vert_src = String::new();
        let mut frag_src = String::new();
        let mut geom_src = String::new();

        enum Target { Vert, Frag, Geom }
        let mut current = None;

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.starts_with("--") {
                let value = line[2..].trim();
                match value {
                    "VERT" => current = Some(Target::Vert),
                    "FRAG" => current = Some(Target::Frag),
                    "GEOM" => current = Some(Target::Geom),
                    _ => {
                        let message = format!("Expected 'VERT', 'FRAG' or 'GEOM', found {}", &line[2..]);
                        return Err(io::Error::new(io::ErrorKind::Other, message));
                    }
                }
            } else {
                match current {
                    Some(Target::Vert) => {
                        vert_src.push_str(line);
                        vert_src.push('\n');
                    },
                    Some(Target::Frag) => {
                        frag_src.push_str(line);
                        frag_src.push('\n');
                    },
                    Some(Target::Geom) => {
                        geom_src.push_str(line);
                        geom_src.push('\n');
                    },
                    None => (),
                }
            }
        }

        Ok(ShaderPrototype {
            vert_src: vert_src,
            geom_src: geom_src,
            frag_src: frag_src
        })
    }

    /// Creates a new shader prototype from the given string code literals.
    pub fn new_prototype(vert_src: &str, geom_src: &str, frag_src: &str) -> ShaderPrototype {
        ShaderPrototype {
            vert_src: String::from(vert_src),
            geom_src: String::from(geom_src),
            frag_src: String::from(frag_src),
        }
    }

    /// Inserts input declarations matching the output declarations of a previous
    /// shader stage into the next shader stage. For example, if the vertex source
    /// contains `out vec4 color;`, `in vec4 color;` will be added to the either 
    /// the geometry or the fragment shader, depending on which one exists.
    pub fn propagate_outputs(&mut self) {
        let vert_out = create_inputs(&self.vert_src);
        if self.geom_src.is_empty() {
            if !self.frag_src.is_empty() {
                self.frag_src = prepend_code(&self.frag_src, &vert_out);
            }
        } else {
            if !self.frag_src.is_empty() {
                let geom_out = create_inputs(&self.geom_src);
                self.frag_src = prepend_code(&self.frag_src, &geom_out);
            }
            self.geom_src = prepend_code(&self.geom_src, &vert_out);
        }
    }

    /// Converts this prototype into a shader
    pub fn build(&self) -> io::Result<Shader> {
        let vert_src = self.vert_src.as_str();
        let frag_src = if self.frag_src.is_empty() { None } else { Some(self.frag_src.as_str()) };
        let geom_src = if self.geom_src.is_empty() { None } else { Some(self.geom_src.as_str()) };

        Shader::new(vert_src, geom_src, frag_src)
    }

    /// Converts this prototype into a shader, inserting input declarations for the given
    /// vertex into the fragment shader
    pub fn build_with_vert<T>(&self) -> io::Result<Shader> where T: Vertex {
        let input_decl = <T as Vertex>::gen_shader_input_decl();
        let vert_src = &prepend_code(&self.vert_src, &input_decl);
        let frag_src = if self.frag_src.is_empty() { None } else { Some(self.frag_src.as_str()) };
        let geom_src = if self.geom_src.is_empty() { None } else { Some(self.geom_src.as_str()) };

        Shader::new(vert_src, geom_src, frag_src)
    }
}

/// A OpenGL shader that is ready for use
#[derive(Debug, Hash)]
pub struct Shader {
    program: GLuint,
    vert_shader: GLuint,
    geom_shader: Option<GLuint>,
    frag_shader: Option<GLuint>
}

impl Shader {
    /// Constructs a glsl shader from source. Note that the geometry and fragment shaders are
    /// optional, as they are not needed for all purposes.
    pub fn new(vert_src: &str,
               geom_src: Option<&str>,
               frag_src: Option<&str>)
               -> io::Result<Shader> {
        let program;
        let vert_shader = 0;
        let frag_shader;
        let geom_shader;

        unsafe {
            program = gl::CreateProgram();

            let vert_shader = compile(vert_src, gl::VERTEX_SHADER)?;
            gl::AttachShader(program, vert_shader);

            geom_shader = {
                if let Some(geom_src) = geom_src {
                    let geom_shader = compile(geom_src, gl::GEOMETRY_SHADER)?;
                    gl::AttachShader(program, geom_shader);
                    Some(geom_shader)
                } else {
                    None
                }
            };

            frag_shader = {
                if let Some(frag_src) = frag_src {
                    let frag_shader = compile(frag_src, gl::FRAGMENT_SHADER)?;
                    gl::AttachShader(program, frag_shader);
                    Some(frag_shader)
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
                gl::DeleteShader(vert_shader);
                if let Some(geom_shader) = geom_shader {
                    gl::DeleteShader(geom_shader);
                }
                if let Some(frag_shader) = frag_shader {
                    gl::DeleteShader(frag_shader);
                }

                let message = str::from_utf8(&buffer).ok().expect("Shader log is not valid utf8").to_string();
                return Err(io::Error::new(io::ErrorKind::Other, message));
            }
        }

        Ok(Shader {
            program: program,
            vert_shader: vert_shader,
            geom_shader: geom_shader,
            frag_shader: frag_shader,
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

    /// Note: Shader needs to be bound before call to this! 
    fn get_uniform_location(&self, uniform_name: &str) -> Option<GLint> {
        unsafe {
            let c_str = CString::new(uniform_name.as_bytes()).unwrap();
            let location = gl::GetUniformLocation(self.program, c_str.as_ptr()); 
            if location == -1 {
                None
            } else {
                Some(location)
            }
        }
    }

    pub fn set_uniform<T: UniformValue>(&self, uniform_name: &str, value: T) {
        if let Some(location) = self.get_uniform_location(uniform_name) {
            self.bind();
            unsafe { value.set_uniform(location); }
        } else {
            println!("Invalid uniform name: {}", uniform_name); // Maybe panic
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            gl::DeleteShader(self.vert_shader);
            if let Some(geom_shader) = self.geom_shader {
                gl::DeleteShader(geom_shader);
            }
            if let Some(frag_shader) = self.frag_shader {
                gl::DeleteShader(frag_shader);
            }
        }
    }
}

/// Prepends the given section of code to the beginning of the given piece of
/// shader src. Note that code is inserted after the `#version ...`
/// preprocessor, if present.
pub fn prepend_code(src: &str, code: &str) -> String {
    let insert_index =
        if let Some(preprocessor_index) = src.find("#version") {
            if let Some(newline_index) = src[preprocessor_index..].find('\n') {
                newline_index + preprocessor_index
            } else {
                // We might want to warn the user in this case. A shader with a
                // #version preprocessor but no newline will (I think) never
                // be valid, unless the code inserted here makes it valid
                src.len() 
            }
        } else {
            0
        };

    let mut result = String::with_capacity(src.len() + code.len() + 2); // +2 for newlines surrounding inserted code

    result.push_str(&src[0..insert_index]);

    result.push('\n');
    result.push_str(code);
    result.push('\n');

    if !src.is_empty() && insert_index < src.len() - 1 {
        result.push_str(&src[insert_index+1..]);
    }

    result
}

/// Finds all variables marked as `out` in the given glsl shader and generates
/// corresponding ´in´ declarations for the next shader stage. These declarations
/// can be inserted into the next stage with `prepend_code()`
///
/// TODO: This does not take the special format needed for geometry shaders into account!
///
/// # Example
/// ```
/// let shader = "
///     #version 330 core
///     out vec4 color;
///     out vec2 tex;
///     // Rest of shader ommited
/// ";
///
/// let inputs = create_inputs(shader);
///
/// assert_eq!("in vec4 color; in vec2 tex;", inputs);
/// ```
pub fn create_inputs(src: &str) -> String {
    let pattern = "out";
    let mut result = String::new();

    let mut i = 0;
    'outer:
    while i < src.len() - pattern.len() - 1{
        // Search for occurences of "out"
        if let Some(index) = src[i..].find(pattern) {
            let index = i + index; // Index will be offset from start
            i = index + pattern.len();

            // Check if the "out" is at the start of a line, or after a semicolon
            for prev in src[0..index].chars().rev() {
                if prev == '\n' || prev == '\r' || prev == ';' {
                    break;
                } else if prev.is_whitespace() {
                    continue;
                }
                continue 'outer
            }
            // We now know that the "out" is actually a keyword, and not a identifier name part
            
            // Find the end of the line, delimited by a semicolon
            let start = index + pattern.len() + 1;
            let end = match src[start..].find(";") {
                Some(end) => start + end,
                None => continue 'outer
            };

            // Append the output to the string
            if !result.is_empty() { result.push(' '); }
            result.push_str("in ");
            result.push_str(&src[start..end]);
            result.push(';');
        } else {
            break 'outer;
        }
    }

    result
}

fn compile(src: &str, shader_type: GLenum) -> io::Result<GLuint> {
    unsafe {
        let shader = gl::CreateShader(shader_type);

        let c_str = CString::new(src.as_bytes()).unwrap();
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
            return Err(io::Error::new(io::ErrorKind::Other, message));
        } else {
            return Ok(shader);
        }
    }
}

/// Everything which implements this trait can be stured into the uniform value
/// of a shader, assuming its implementation is valid
pub trait UniformValue {
    unsafe fn set_uniform(&self, location: GLint); 
}
// Implementations for vectors and matricies
impl UniformValue for Vec2<f32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2f(location, self.x, self.y); } }
impl UniformValue for Vec2<f64> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2f(location, self.x as f32, self.y as f32); } }
impl UniformValue for Vec2<i32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2i(location, self.x, self.y); } }
impl UniformValue for Vec2<u32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2ui(location, self.x, self.y); } }
impl UniformValue for Vec3<f32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3f(location, self.x, self.y, self.z); } }
impl UniformValue for Vec3<f64> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3f(location, self.x as f32, self.y as f32, self.z as f32); } }
impl UniformValue for Vec3<i32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3i(location, self.x, self.y, self.z); } }
impl UniformValue for Vec3<u32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3ui(location, self.x, self.y, self.z); } }
impl UniformValue for Vec4<f32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4f(location, self.x, self.y, self.z, self.w); } }
impl UniformValue for Vec4<f64> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4f(location, self.x as f32, self.y as f32, self.z as f32, self.w as f32); } }
impl UniformValue for Vec4<i32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4i(location, self.x, self.y, self.z, self.w); } }
impl UniformValue for Vec4<u32> { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4ui(location, self.x, self.y, self.z, self.w); } }
impl UniformValue for Mat4<f32> { unsafe fn set_uniform(&self, location: GLint) { gl::UniformMatrix4fv(location, 1, false as GLboolean, &(self.a11) as *const GLfloat); } }
// Implementations for f32, i32 and u32 single values and tuples.
impl UniformValue for f32                   { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform1f(location, *self as GLfloat); } }
impl UniformValue for (f32, f32)            { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2f(location, (*self).0 as GLfloat, (*self).1 as GLfloat); } }
impl UniformValue for (f32, f32, f32)       { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3f(location, (*self).0 as GLfloat, (*self).1 as GLfloat, (*self).2 as GLfloat); } }
impl UniformValue for (f32, f32, f32, f32)  { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4f(location, (*self).0 as GLfloat, (*self).1 as GLfloat, (*self).2 as GLfloat, (*self).3 as GLfloat); } }
impl UniformValue for i32                   { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform1i(location, *self as GLint); } }
impl UniformValue for (i32, i32)            { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2i(location, (*self).0 as GLint, (*self).1 as GLint); } }
impl UniformValue for (i32, i32, i32)       { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3i(location, (*self).0 as GLint, (*self).1 as GLint, (*self).2 as GLint); } }
impl UniformValue for (i32, i32, i32, i32)  { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4i(location, (*self).0 as GLint, (*self).1 as GLint, (*self).2 as GLint, (*self).3 as GLint); } }
impl UniformValue for u32                   { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform1ui(location, *self as GLuint); } }
impl UniformValue for (u32, u32)            { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform2ui(location, (*self).0 as GLuint, (*self).1 as GLuint); } }
impl UniformValue for (u32, u32, u32)       { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform3ui(location, (*self).0 as GLuint, (*self).1 as GLuint, (*self).2 as GLuint); } }
impl UniformValue for (u32, u32, u32, u32)  { unsafe fn set_uniform(&self, location: GLint) { gl::Uniform4ui(location, (*self).0 as GLuint, (*self).1 as GLuint, (*self).2 as GLuint, (*self).3 as GLuint); } }

/// Shorthand for loading a shader, propagating its outputs and inserting input declarations
/// for a given vertex type
///
/// # Exapmple
/// ```
/// TODO
/// ```
#[macro_export]
macro_rules! load_shader {
    ($src:expr, $vert:ty) => {
        {
            match ShaderPrototype::from_file($src) {
                Ok(mut shader) => {
                    shader.propagate_outputs();
                    shader.build_with_vert::<$vert>()
                },
                Err(err) => Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs() {
        let shader = "
            #version 330 core
            out vec4 color;
            out vec2 tex;
            // Rest of shader ommited
        ";

        let inputs = create_inputs(shader);

        assert_eq!("in vec4 color; in vec2 tex;", inputs);
    }
}

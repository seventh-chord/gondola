
//! Utilities for loading and using glsl shaders.
//!
//! [`ShaderPrototype`](struct.ShaderPrototype.html) is used to load and
//! modify the source of a shader. It can then be converted to an actual
//! [`Shader`](struct.Shader.html) which can be used for rendering.

use std::{ptr, str, fmt, error, io};
use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader};
use std::ffi::CString;
use gl;
use gl::types::*;
use buffer::Vertex;
use cable_math::{Mat4, Vec2, Vec3, Vec4};

/// A shader that has not yet been fully compiled
#[derive(Debug)]
pub struct ShaderPrototype {
    vert_src: String,
    frag_src: String,
    geom_src: String,
    bind_to_matrix_storage: bool,
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
    pub fn from_file<P>(path: P) -> Result<ShaderPrototype, ShaderError> where P: AsRef<Path> {
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
                        return Err(ShaderError::FileFormat(message));
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
            frag_src: frag_src,
            bind_to_matrix_storage: false,
        })
    }

    /// Creates a new shader prototype from the given string code literals.
    pub fn new_prototype(vert_src: &str, geom_src: &str, frag_src: &str) -> ShaderPrototype {
        ShaderPrototype {
            vert_src: String::from(vert_src),
            geom_src: String::from(geom_src),
            frag_src: String::from(frag_src),
            bind_to_matrix_storage: false,
        }
    }

    /// Inserts input declarations matching the output declarations of a previous
    /// shader stage into the next shader stage. For example, if the vertex source
    /// contains `out vec4 color;`, `in vec4 color;` will be added to the either 
    /// the geometry or the fragment shader, depending on which one exists.
    pub fn propagate_outputs(&mut self) {
        if self.geom_src.is_empty() {
            let vert_out = create_inputs(&self.vert_src, false);
            if !self.frag_src.is_empty() {
                self.frag_src = prepend_code(&self.frag_src, &vert_out);
            }
        } else {
            if !self.frag_src.is_empty() {
                let geom_out = create_inputs(&self.geom_src, false);
                self.frag_src = prepend_code(&self.frag_src, &geom_out);
            }
            
            let vert_out = create_inputs(&self.vert_src, true);
            self.geom_src = prepend_code(&self.geom_src, &vert_out);
        }
    }

    /// Binds this shader to matrix stack storage, so that it automatically
    /// has access to the currently set matrix stacks without the need to 
    /// set uniforms every time a shader is bound.
    ///
    /// *Implementation note*: Matricies are stored at the last valid uniform
    /// buffer binding index.
    pub fn bind_to_matrix_storage(&mut self) {
        let uniform_block_decl = "layout(shared,std140) uniform MatrixBlock { mat4 mvp; };";
        if self.geom_src.is_empty() {
            self.vert_src = prepend_code(&self.vert_src, uniform_block_decl);
        } else {
            self.geom_src = prepend_code(&self.geom_src, uniform_block_decl);
        }
        self.bind_to_matrix_storage = true;
    }


    /// Converts this prototype into a shader
    pub fn build(&self) -> Result<Shader, ShaderError> {
        let vert_src = self.vert_src.as_str();
        let frag_src = if self.frag_src.is_empty() { None } else { Some(self.frag_src.as_str()) };
        let geom_src = if self.geom_src.is_empty() { None } else { Some(self.geom_src.as_str()) };

        match Shader::new(vert_src, geom_src, frag_src) {
            Ok(shader) => {
                if self.bind_to_matrix_storage {
                    unsafe {
                        let binding_index = ::matrix_stack::get_uniform_binding_index();
                        let c_str = CString::new("MatrixBlock").unwrap();
                        let block_index = gl::GetUniformBlockIndex(shader.program, c_str.as_ptr());
                        gl::UniformBlockBinding(shader.program, block_index, binding_index);
                    }
                }
                Ok(shader)
            },
            Err(err) => Err(err)
        }
    }

    /// Converts this prototype into a shader, inserting input declarations for the given
    /// vertex into the fragment shader
    pub fn build_with_vert<T>(&self) -> Result<Shader, ShaderError> where T: Vertex {
        let input_decl = <T as Vertex>::gen_shader_input_decl();
        let vert_src = &prepend_code(&self.vert_src, &input_decl);
        let frag_src = if self.frag_src.is_empty() { None } else { Some(self.frag_src.as_str()) };
        let geom_src = if self.geom_src.is_empty() { None } else { Some(self.geom_src.as_str()) };

        match Shader::new(vert_src, geom_src, frag_src) {
            Ok(shader) => {
                if self.bind_to_matrix_storage {
                    unsafe {
                        let binding_index = ::matrix_stack::get_uniform_binding_index();
                        let c_str = CString::new("MatrixBlock").unwrap();
                        let block_index = gl::GetUniformBlockIndex(shader.program, c_str.as_ptr());
                        gl::UniformBlockBinding(shader.program, block_index, binding_index);
                    }
                }
                Ok(shader)
            },
            Err(err) => Err(err)
        }
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
               -> Result<Shader, ShaderError> {
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

                let message = str::from_utf8(&buffer).expect("Shader log was not valid UTF-8").to_string();
                return Err(ShaderError::Link(message));
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
fn prepend_code(src: &str, code: &str) -> String {
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
/// can be inserted into the next stage with `prepend_code()`.
///
/// Note that this takes the format required for geometry shaders into account. If
/// `for_geom` is set to `true` inputs will be marked as arrays.
///
/// # Example
/// ```
/// use gondola::shader::create_inputs;
///
/// let shader = "
///     #version 330 core
///     out vec4 color;
///     out vec2 tex;
///     // Rest of shader ommited
/// ";
///
/// let inputs = create_inputs(shader, false);
///
/// assert_eq!("in vec4 color; in vec2 tex;", inputs);
/// ```
pub fn create_inputs(src: &str, for_geom: bool) -> String {
    let patterns = [
        ("out", "in"),
        ("flat out", "flat in"),
    ];
    let mut result = String::new();

    let mut i = 0;
    'outer:
    while i < src.len() - 1{
        // Search for occurences of "out" and other patterns
        let next = patterns.iter()
            // Search for each pattern
            .map(|pair| (src[i..].find(pair.0), *pair))
            // Eliminate those that where not found at all
            .filter(|&(index, _)| index.is_some())
            // Find the first one
            .min_by_key(|pair| pair.0);

        if let Some((Some(index), (pattern, input))) = next {
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
            result.push_str(input);
            result.push(' ');
            result.push_str(&src[start..end]);
            if for_geom { result.push_str("[]"); }
            result.push(';');
        } else {
            break 'outer;
        }
    }

    result
}

fn compile(src: &str, shader_type: GLenum) -> Result<GLuint, ShaderError> {
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
            let message = format!("{}For source: \"\n{}\"",
                                  message, src);
            return Err(ShaderError::Compile(message));
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
/// # Parameters
/// `load_shader!(src, vertex_type)`
///
/// * `src`: The source file from which to load this shader. Should be `AsRef<Path>` 
///   (This includes `&str` and `Path`).
/// * `vertex_type`: The name of a struct which implementes [`Vertex`](buffer/trait.Vertex.html).
///
/// # Example
/// ```rust,ignore
/// # #![allow(dead_code, unused_variables)]
/// #[macro_use]
/// extern crate gondola; 
/// #[macro_use]
/// extern crate gondola_derive;
/// extern crate gl; // Required for the vertex derive
///
/// use gondola::shader::*;
/// use gondola::buffer::Vertex;
/// 
/// #[derive(Vertex)]
/// struct TestVertex {
///     position: (f32, f32),
/// }
///
/// # fn main() {
/// let shader = load_shader!("assets/basic.glsl", TestVertex)?;
/// # }
/// ```
#[macro_export]
macro_rules! load_shader {
    ($src:expr, $vert:ty) => {
        ::gondola::shader::ShaderPrototype::from_file($src).and_then(|mut prototype| {
            prototype.propagate_outputs();
            prototype.bind_to_matrix_storage();
            prototype.build_with_vert::<$vert>()
        })
    };
}

/// Errors which can occur in the various stages of shader creation.
#[derive(Debug)]
pub enum ShaderError {
    Compile(String),
    Link(String),
    FileFormat(String),
    Io(io::Error),
}

impl error::Error for ShaderError {
    fn description(&self) -> &str {
        match *self {
            ShaderError::Compile(ref log)       => log,
            ShaderError::Link(ref log)          => log,
            ShaderError::FileFormat(ref msg)    => msg,
            ShaderError::Io(ref err)            => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        if let ShaderError::Io(ref err) = *self {
            err.cause()
        } else {
            None
        }
    }
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ShaderError::Compile(ref log)       => write!(f, "Compile error: \n{}\n", log),
            ShaderError::Link(ref log)          => write!(f, "Link error: \n{}\n", log),
            ShaderError::FileFormat(ref msg)    => write!(f, "File format error: {}", msg),
            ShaderError::Io(ref err)            => write!(f, "Io error while loading shader: {}", err),
        }
    }
}

impl From<io::Error> for ShaderError {
    fn from(err: io::Error) -> ShaderError {
        ShaderError::Io(err)
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
            flat out ivec2 tile;
            out vec2 tex;
            // Rest of shader ommited
        ";

        let inputs = create_inputs(shader, false);
        assert_eq!("in vec4 color; flat in ivec2 tile; in vec2 tex;", inputs);

        let geom_inputs = create_inputs(shader, true);
        assert_eq!("in vec4 color[]; flat in ivec2 tile[]; in vec2 tex[];", geom_inputs);
    }
}


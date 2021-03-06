
//! Utilities for loading and using glsl shaders.
//!
//! [`ShaderPrototype`](struct.ShaderPrototype.html) is used to load and
//! modify the source of a shader. It can then be converted to an actual
//! [`Shader`](struct.Shader.html) which can be used for rendering.

use std::{mem, ptr, str, fmt, error, io};
use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader};
use std::ffi::CString;
use std::borrow::Borrow;

use gl;
use gl::types::*;

use util;
use buffer::Vertex;

mod uniform;
pub use self::uniform::{UniformValue, UniformKind, UniformBinding};

/// A shader that has not yet been fully compiled
pub struct ShaderPrototype {
    vert_src: String,
    frag_src: String,
    geom_src: String,
    transform_feedback_outputs: Option<Vec<String>>,
}

impl ShaderPrototype {
    /// Loads a shader from a file. The file should contain all the shader stages, with
    /// each shader stage prepended by `-- name`, where name is one of `VERT`, `FRAG`
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
            vert_src,
            geom_src,
            frag_src,
            transform_feedback_outputs: None,
        })
    }

    /// Creates a new shader prototype from the given string code literals.
    pub fn new_prototype(vert_src: &str, geom_src: &str, frag_src: &str) -> ShaderPrototype {
        ShaderPrototype {
            vert_src: vert_src.to_owned(),
            geom_src: geom_src.to_owned(),
            frag_src: frag_src.to_owned(),
            transform_feedback_outputs: None,
        }
    }

    /// Inserts input declarations matching the output declarations of a previous shader stage into 
    /// the next shader stage. For example, if the vertex source contains `out vec4 color;`, 
    /// `in vec4 color;` will be added to the either the geometry or the fragment shader, depending 
    /// on which one exists.
    pub fn propagate_outputs(&mut self) {
        if self.geom_src.is_empty() {
            let vert_out = create_inputs(&self.vert_src, false);
            if !self.frag_src.is_empty() {
                prepend_code(&mut self.frag_src, &vert_out);
            }
        } else {
            if !self.frag_src.is_empty() {
                let geom_out = create_inputs(&self.geom_src, false);
                prepend_code(&mut self.frag_src, &geom_out);
            }
            
            let vert_out = create_inputs(&self.vert_src, true);
            prepend_code(&mut self.geom_src, &vert_out);
        }
    }

    /// Adds input declarations for the given vertex to this shader. The generated shader can then be 
    /// used to draw [`VertexBuffer`]s with vertices of type `T`.
    /// [`VertexBuffer`]: ../buffer/struct.VertexBuffer.html
    pub fn with_input_vert<T>(&mut self, name_prefix: &str) where T: Vertex {
        let input = <T as Vertex>::gen_shader_input_decl(name_prefix);
        prepend_code(&mut self.vert_src, &input);
    }

    /// Adds output declarations for the given vertex to this shader. This is intended for usage
    /// with transform feedback. The generated shader can then be used as a target for
    /// [`transform_feedback_into`][1]
    ///
    /// [1]: ../buffer/struct.VertexBuffer.html#method.transform_feedback_into
    pub fn with_transform_output_vert<T>(&mut self, name_prefix: &str) where T: Vertex {
        let output = <T as Vertex>::gen_transform_feedback_decl(name_prefix);
        prepend_code(&mut self.vert_src, &output);

        self.transform_feedback_outputs = Some(<T as Vertex>::gen_transform_feedback_outputs(name_prefix));
    }

    /// Converts this prototype into a shader
    pub fn build(&self) -> Result<Shader, ShaderError> {
        let vert_src = self.vert_src.as_str();
        let frag_src = if self.frag_src.is_empty() { None } else { Some(self.frag_src.as_str()) };
        let geom_src = if self.geom_src.is_empty() { None } else { Some(self.geom_src.as_str()) };

        Shader::new(vert_src, geom_src, frag_src, self.transform_feedback_outputs.clone())
    }
}

/// A OpenGL shader that is ready for use
pub struct Shader {
    program: GLuint,
    uniforms: Vec<UniformBinding>,
}

impl Shader {
    fn new(
        vert_src: &str,
        geom_src: Option<&str>,
        frag_src: Option<&str>,
        transform_feedback_outputs: Option<Vec<String>>
    ) -> Result<Shader, ShaderError> 
    {
        let program;
        let mut uniforms;

        unsafe {
            program = gl::CreateProgram();

            let vert_shader = compile(vert_src, gl::VERTEX_SHADER)?;
            gl::AttachShader(program, vert_shader);

            let geom_shader = {
                if let Some(geom_src) = geom_src {
                    let geom_shader = compile(geom_src, gl::GEOMETRY_SHADER)?;
                    gl::AttachShader(program, geom_shader);

                    Some(geom_shader)
                } else {
                    None
                }
            };

            let frag_shader = {
                if let Some(frag_src) = frag_src {
                    let frag_shader = compile(frag_src, gl::FRAGMENT_SHADER)?;
                    gl::AttachShader(program, frag_shader);

                    Some(frag_shader)
                } else {
                    None
                }
            };

            if let Some(transform_feedback_outputs) = transform_feedback_outputs {
                let names = transform_feedback_outputs.into_iter()
                    .map(|s| CString::new(s.into_bytes()).unwrap())
                    .collect::<Vec<_>>();
                let name_ptrs = names.iter()
                    .map(|n| n.as_ptr())
                    .collect::<Vec<_>>();

                gl::TransformFeedbackVaryings(program, name_ptrs.len() as GLsizei, name_ptrs.as_ptr(), gl::INTERLEAVED_ATTRIBS);
            }

            gl::LinkProgram(program);

            // The specification says that DeleteShader marks the shader as disposable, but does
            // not delete it until the program is deleted.
            gl::DeleteShader(vert_shader);
            if let Some(geom_shader) = geom_shader {
                gl::DeleteShader(geom_shader);
            }
            if let Some(frag_shader) = frag_shader {
                gl::DeleteShader(frag_shader);
            }

            // Handle errors
            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status != (gl::TRUE as GLint) {
                let mut log_len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len);

                let mut buffer = Vec::with_capacity(log_len as usize);
                buffer.set_len((log_len as usize) - 1); // Skip null terminator
                gl::GetProgramInfoLog(program, log_len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);

                gl::DeleteProgram(program);

                let message = str::from_utf8(&buffer).expect("Shader log was not valid UTF-8").to_string();
                let message = format!(
                    "{}\nFor source:\n-- VERT\n{}\n-- FRAG\n{}\n-- GEOM\n{}",
                    message,
                    vert_src,
                    geom_src.unwrap_or(""),
                    frag_src.unwrap_or(""),
                );
                return Err(ShaderError::Link(message));
            } 

            // Load uniforms
            let mut uniform_count = 0;
            gl::GetProgramiv(program, gl::ACTIVE_UNIFORMS, &mut uniform_count);

            uniforms = Vec::with_capacity(uniform_count as usize);

            for index in 0..uniform_count {
                const MAX_NAME_LENGTH: usize = 512;

                let mut name_length = 0;
                let mut name_buffer = [0u8; MAX_NAME_LENGTH];

                let mut size = 0;
                let mut kind = 0;

                gl::GetActiveUniform(
                    program, index as u32,
                    MAX_NAME_LENGTH as i32,
                    &mut name_length,
                    &mut size,
                    &mut kind,
                    name_buffer.as_mut_ptr() as *mut i8,
                );

                let location = gl::GetUniformLocation(
                    program,
                    name_buffer.as_ptr() as *const i8
                );

                // As far as i can tell, glsl identifiers are only allowed to contain a..z, A..Z,
                // 0..9 and underscores. Therefore, this conversion is just fine
                let name = util::ascii_to_string(&name_buffer[.. (name_length as usize)]);

                let kind: UniformKind = mem::transmute(kind);

                uniforms.push(UniformBinding { name, location, kind });
            }
        }

        Ok(Shader {
            program,
            uniforms,
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

    fn get_uniform_binding(&self, name: &str) -> Option<&UniformBinding> {
        for binding in self.uniforms.iter() {
            if binding.name == name {
                return Some(binding);
            }
        }

        return None;
    }

    /// Sets the uniform with the given name to the given value. This prints a warning if no
    /// uniform with the given name exists.
    ///
    /// This binds this shader if the given uniform exists!
    pub fn set_uniform<T, U>(&self, uniform_name: &str, value: U) 
      where T: UniformValue,
            U: Borrow<T>,
    {
        self.set_uniform_with_offset(uniform_name, 0, value);
    }

    /// Sets the uniform at the given offset from the given name to the given value. When a uniform
    /// is an array this can be used to set a specific element of that array. For example, if the
    /// shader contains `uniform vec3 positions[2];`, `set_uniform_with_offset(1, "positions", ...)`
    /// will modify the second elment of the positions array.  This prints a warning if no uniform 
    /// with the given name exists.
    ///
    /// This binds this shader if the given uniform exists!
    pub fn set_uniform_with_offset<T, U>(&self, uniform_name: &str, offset: usize, value: U) 
      where T: UniformValue,
            U: Borrow<T>,
    {
        if let Some(binding) = self.get_uniform_binding(uniform_name) {
            let value_kind = T::KIND;
            if binding.kind != value_kind {
                panic!(
                    "Tried to set uniform \"{}\" to a `{}`, but the uniform has type `{}`",
                    binding.name, value_kind, binding.kind,
                );
            } else {
                self.bind();
                unsafe { T::set_uniform(value.borrow(), binding.location + offset as GLint); }
            }
        } else {
            // The reason we simply print a error here is because it sometimes is convenient to
            // ignore a uniform while refactoring a shader. panicking or returning some result would
            // force changing rust code when glsl code is changed, which slows down the development
            // process.
            println!("Invalid uniform name: {}", uniform_name); 
        }
    }

    /// Sets the uniform with the given name to the given slice of values. Note that this expects
    /// the uniform with the given name to be a array. This prints a warning if no uniform with the 
    /// given name exists.
    ///
    /// This binds this shader if the given uniform exists!
    pub fn set_uniform_slice<T>(&self, uniform_name: &str, slice: &[T]) 
      where T: UniformValue,
    {
        if let Some(binding) = self.get_uniform_binding(uniform_name) {
            let value_kind = T::KIND;
            if binding.kind != value_kind {
                panic!(
                    "Tried to set uniform \"{}\" to a `{}`, but the uniform has type `{}`",
                    binding.name, value_kind, binding.kind,
                );
            } else {
                self.bind();
                unsafe { T::set_uniform_slice(slice, binding.location); }
            }
        } else {
            // The reason we simply print a error here is because it sometimes is convenient to
            // ignore a uniform while refactoring a shader. panicking or returning some result would
            // force changing rust code when glsl code is changed, which slows down the development
            // process.
            println!("Invalid uniform name: {}", uniform_name); 
        }
    }

    /// Sets up the uniform block with the given name to retrieve data from the given binding
    /// index. A [`PrimitiveBuffer`] with `BufferTarget::Uniform` can then be bound to that same
    /// index using [`PrimitiveBuffer::bind_base(matrix_binding)`]. The data in that buffer can
    /// then be accessed from the uniform block in the shader.
    ///
    /// Using uniform blocks with uniform buffers is usefull, as the same data can be accessed
    /// by multiple shaders.
    ///
    /// OpenGL is required to support at least 36 binding indices.
    ///
    /// # Example
    ///
    /// In `shader.glsl`:
    ///
    /// ```glsl 
    /// layout(shared,std140) // These layout qualifiers are needed
    /// uniform matrix_block { 
    ///     mat4 model_view_projection_matrix; 
    /// };
    ///
    /// void main() {
    ///     gl_Position = model_view_projection_matrix * vec4(...);
    /// }
    /// ```
    ///
    /// In `main.rs`:
    ///
    /// ```rust,ignore
    /// let shader = load_shader!("shader.glsl", ...); 
    /// shader.bind_uniform_block("matrix_block", 23); // `matrix_block` now gets data from index 23.
    ///
    /// let buffer = PrimitiveBuffer::new(BufferTarget::Uniform, BufferUsage::DynamicDraw);
    /// buffer.bind_base(23); // `buffer` now gives data to index 23.
    ///
    /// buffer.put_at_start(Mat4::ortho( ... )); 
    /// // The shader now sees `model_view_projection_matrix` as a orthographic projection matrix.
    /// ```
    /// 
    /// [`PrimitiveBuffer`]: ../buffer/struct.PrimitiveBuffer.html
    /// [`PrimitiveBuffer::bind_base(matrix_binding)`]: ../buffer/struct.PrimitiveBuffer.html#method.bind_base
    pub fn bind_uniform_block(&self, block_name: &str, binding_index: usize) {
        unsafe {
            let c_str = CString::new(block_name).unwrap();
            let block_index = gl::GetUniformBlockIndex(self.program, c_str.as_ptr());
            if block_index == gl::INVALID_INDEX {
                println!("Invalid uniform");
            } else {
                gl::UniformBlockBinding(self.program, block_index, binding_index as GLuint);
            }
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
        }
    }
}

/// Prepends the given section of code to the beginning of the given piece of
/// shader src. Note that code is inserted after the `#version ...`
/// preprocessor, if present.
fn prepend_code(src: &mut String, code: &str) {
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

    src.insert(insert_index, '\n');
    src.insert_str(insert_index + 1, code);
    src.insert(insert_index + 1 + code.len(), '\n');
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

/// Shorthand for loading a shader, propagating its outputs and inserting input declarations
/// for a given vertex type. 
///
/// This macro allways returns `Result<Shader, ShaderError>`.
///
/// # Parameters
/// `load_shader!(src, vertex_type)`
///
/// * `src`: The source file from which to load this shader. Should be `AsRef<Path>` 
///   (This includes `&str` and `Path`).
/// * `vertex_type`: The name of a struct which implementes [`Vertex`].
///
/// `load_shader!(src, vertex_type => transform_output_type)`
///
/// * `src`: The source file from which to load this shader. Should be `AsRef<Path>` 
///   (This includes `&str` and `Path`).
/// * `vertex_type`: The name of a struct which implementes [`Vertex`].
/// * `transform_output_type`: The name of a struct which implements [`Vertex`]. This
///    shader can then be used for transform feedback, with `transform_output_type` as a target
///    type. Output declarations for this vertex type will also be inserted.
///
/// # Prefixes
/// When a shader has many inputs and outputs it can be usefull to prefix all variables from a
/// given source with a common prefix, to avoid naming conflicts. This can be done by appending
/// `: "prefix"` to the vertex type. 
///
/// For example, if 
///
/// `load_shader!("...", Vert)` generates `layout(...) in vec3 position`
///
/// then
///
/// `load_shader!("...", Vert: "in_")` generates `layout(...) in vec3 in_position`
///
/// [`Vertex`]: buffer/trait.Vertex.html
///
/// # Example
/// ```rust,ignore
/// # #![allow(dead_code, unused_variables)]
/// #[macro_use]
/// extern crate gondola; 
/// #[macro_use]
/// extern crate gondola_derive;
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
    // Aliases for shorter formats
    ($src:expr, $vert:ty) => {
        load_shader!($src, $vert: "")
    };
    ($src:expr, $vert:ty => $target:ty) => {
        load_shader!($src, $vert => $target: "out_");
    };
    ($src:expr, $vert:ty => $target:ty: $target_prefix:expr) => {
        load_shader!($src, $vert: "" => $target: $target_prefix);
    };

    // With custom prefixes
    ($src:expr, $vert:ty: $vert_prefix:expr) => {
        ::gondola::shader::ShaderPrototype::from_file($src).and_then(|mut prototype| {
            prototype.propagate_outputs();
            prototype.with_input_vert::<$vert>($vert_prefix);
            prototype.build()
        })
    };
    ($src:expr, $vert:ty: $vert_prefix:expr => $target:ty: $target_prefix:expr) => {
        ::gondola::shader::ShaderPrototype::from_file($src).and_then(|mut prototype| {
            prototype.propagate_outputs();
            prototype.with_input_vert::<$vert>($vert_prefix);
            prototype.with_transform_output_vert::<$target>($target_prefix);
            prototype.build()
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


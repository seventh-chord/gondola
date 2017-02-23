
//! Various utilities for rendering and resource management

pub mod graphics {
    //! Wrappers for unsafe OpenGL calls

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

    /// If passed `Some` enables the given blend settings, if passed `None` disables
    /// blending
    pub fn set_blending(blending: Option<BlendSettings>) {
        unsafe {
            if let Some(ref settings) = blending {
                settings.enable();
            } else {
                gl::Disable(gl::BLEND);
            }
        }
    }

    /// Settings used to define OpenGL blend state. You should create a pair of settings
    /// for every operation which uses blending, and apply those settings before rendering.
    /// Blending can be enabled either through
    /// [`enable()`](struct.BlendSettings.html#method.enable)
    /// or
    /// [`graphics::set_blending(Some(my_settings))`](fn.set_blending.html)
    /// and can be disabled wtih
    /// [`graphics::set_blending(None)`](fn.set_blending.html).
    ///
    /// Note that this struct implements `Default`, so default blend settings can be retrieved
    /// with `BlendSettings::default()`.
    #[derive(Debug, Clone, Copy)]
    pub struct BlendSettings {
        pub src_color: BlendFactor,
        pub src_alpha: BlendFactor,
        pub dst_color: BlendFactor,
        pub dst_alpha: BlendFactor,
        pub function: BlendFunction,
    }
    const DEFAULT_BLEND_SETTINGS: BlendSettings = BlendSettings {
        src_color: BlendFactor::SrcAlpha,
        dst_color: BlendFactor::OneMinusSrcAlpha,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::Zero,
        function: BlendFunction::Add,
    };
    impl BlendSettings {
        /// Enables blending, and uses these blend settings 
        pub fn enable(&self) {
            unsafe {
                gl::Enable(gl::BLEND);
                gl::BlendFuncSeparate(self.src_color as GLenum,
                                      self.dst_color as GLenum,
                                      self.src_alpha as GLenum,
                                      self.dst_alpha as GLenum);
                gl::BlendEquation(self.function as GLenum);
            }
        }
    }
    impl Default for BlendSettings {
        fn default() -> BlendSettings {
            DEFAULT_BLEND_SETTINGS
        }
    }

    /// OpenGL blend functions
    #[derive(Copy, Clone, Debug)]
    pub enum BlendFactor {
        Zero                    = gl::ZERO as isize,
        One                     = gl::ONE as isize,
        SrcColor                = gl::SRC_COLOR as isize,
        OneMinusSrcColor        = gl::ONE_MINUS_SRC_COLOR as isize,
        DstColor                = gl::DST_COLOR as isize,
        OneMinusDstColor        = gl::ONE_MINUS_DST_COLOR as isize,
        SrcAlpha                = gl::SRC_ALPHA as isize,
        OneMinusSrcAlpha        = gl::ONE_MINUS_SRC_ALPHA as isize,
        DstAlpha                = gl::DST_ALPHA as isize,
        OneMinusDstAlpha        = gl::ONE_MINUS_DST_ALPHA as isize,
        ConstantColor           = gl::CONSTANT_COLOR as isize,
        OneMinusConstantColor   = gl::ONE_MINUS_CONSTANT_COLOR as isize,
        ConstantAlpha           = gl::CONSTANT_ALPHA as isize,
        OneMinusConstantAlpha   = gl::ONE_MINUS_CONSTANT_ALPHA as isize,
    }
    /// OpenGL blend equations
    #[derive(Copy, Clone, Debug)]
    pub enum BlendFunction {
        /// `Src + Dst`
        Add             = gl::FUNC_ADD as isize,
        /// `Src - Dst`
        Subtract        = gl::FUNC_SUBTRACT as isize,
        /// `Dst - Src`
        ReverseSubtract = gl::FUNC_REVERSE_SUBTRACT as isize,
        /// `min(Dst, Src)`
        Min             = gl::MIN as isize,
        /// `max(Dst, Src)`
        Max             = gl::MAX as isize,
    }
}

pub mod loading {
    //! Utilities for loading/reloading files 

    use std::time::SystemTime;
    use std::fs;
    use std::io;
    use std::path::{PathBuf, Path};
    use std::collections::HashMap;

    /// A utility to execute some action when a file has been changed. Files are checked by
    /// periodically calling [`check(...)`](fn.modified_since.html) inside e.g. the main game
    /// loop.
    pub struct ResourceRefresher {
        last_load_times: HashMap<PathBuf, SystemTime>,
    }

    impl ResourceRefresher {
        pub fn new() -> ResourceRefresher {
            ResourceRefresher {
                last_load_times: HashMap::new(),
            }
        }

        /// Returns true if the given file has been modified since the last call to this function.
        /// If an error occurs while checking if the file has been modified this function prints a
        /// error and returns `false`.
        ///
        /// # Example
        /// ```
        /// use util::loading::ResourceRefresher;
        ///
        /// let mut refresher = ResourceRefresher::new();
        ///
        /// // In main loop
        /// if refresher.check("assets/basic.glsl") {
        ///     // Reload shader
        /// }
        /// ```
        pub fn check<P>(&mut self, path: P) -> bool where P: AsRef<Path> {
            let changed = {
                let key = path.as_ref().to_path_buf();
                let load_time = self.last_load_times.entry(key).or_insert(SystemTime::now());

                match modified_since(path.as_ref(), *load_time) {
                    Ok(value) => value,
                    Err(err) => {
                        println!("Failed to check \"{}\" for modification: {}", path.as_ref().to_string_lossy(), err);
                        false
                    }
                }
            };
            if changed {
                let key = path.as_ref().to_path_buf();
                self.last_load_times.insert(key, SystemTime::now());
            }
            changed
        }
    }

    /// Checks if the given file has been modified since the given time.
    pub fn modified_since(path: &Path, last_time: SystemTime) -> io::Result<bool> {
        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?;

        if let Err(_) = last_time.duration_since(last_modified) {
            return Ok(true);
        } else {
            return Ok(false);
        }
    }
}


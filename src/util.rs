
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


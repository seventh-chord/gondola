
use std::time::SystemTime;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::collections::HashMap;

use shader::{Shader, ShaderPrototype};
use buffer::Vertex;
use texture::Texture;

pub struct ResourceLoader {
    resources: HashMap<PathBuf, Resource>,
}

#[derive(Debug)]
pub enum Resource {
    Shader(ShaderPrototype),
    Texture(Texture),
}

impl ResourceLoader {
    pub fn new(path: &str, _hotload: bool) -> io::Result<ResourceLoader> {
        // TODO: Hotloading
        let root = PathBuf::from(path);
        let mut loader = ResourceLoader {
            resources: HashMap::new(),
        };
        loader.load_dir(&root)?;
        Ok(loader)
    }

    fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        // Find all files in the root directory
        let iter = dir.read_dir()?;
        for entry in iter {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir(&path)?;
            } else if path.is_file() {
                self.load_asset(&path)?;
            } else {
                println!("Invalid path: '{}'", dir.to_string_lossy());
            }
        }
        Ok(())
    }
    fn load_asset(&mut self, file: &Path) -> io::Result<()> {
        let extension = file.extension();
        match extension {
            Some(os_str) => match os_str.to_str() {
                Some("png") => self.load_texture(file)?,
                Some("glsl") => self.load_shader(file)?,
                _ => (),
            },
            _ => (),
        }
        Ok(())
    }
    fn load_shader(&mut self, file: &Path) -> io::Result<()> {
        let mut prototype = ShaderPrototype::from_file(file)?;
        prototype.propagate_outputs();
        self.resources.insert(PathBuf::from(file), Resource::Shader(prototype));

        Ok(())
    }
    fn load_texture(&mut self, file: &Path) -> io::Result<()> {
        let texture = match Texture::load(file) {
            Ok(texture) => texture,
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };
        self.resources.insert(PathBuf::from(file), Resource::Texture(texture));
        Ok(())
    }

    /// Looks for a pre-loaded shader at the given path
    pub fn get_shader<P>(&self, name: P) -> Option<&ShaderPrototype> where P: AsRef<Path> {
        match self.resources.get(name.as_ref()) {
            Some(&Resource::Shader(ref prototype)) => {
                Some(prototype)
            },
            _ => None
        }
    }

    /// Retrives a pre-loaded shader from the given path and builds it for useage with
    /// the given vertex.
    pub fn get_shader_with_vert<T>(&self, name: &str) -> Result<Shader, String> where T: Vertex {
        match self.get_shader(name) {
            Some(ref prototype) => {
                let shader = prototype.build_with_vert::<T>()?;
                Ok(shader)
            },
            None => Err(format!("No such shader: '{}'", name))
        }
    }

    /// Retrieves a pre-loaded texture
    pub fn get_texture<P>(&self, name: P) -> Option<&Texture> where P: AsRef<Path> {
        match self.resources.get(name.as_ref()) {
            Some(&Resource::Texture(ref texture)) => {
                Some(texture)
            },
            _ => None
        }
    }
}

/// A utility to detect changes in files
pub struct WatchPath {
    pub path: PathBuf, 
    last_check: SystemTime,
}

pub struct Watcher {
    paths: Vec<WatchPath>,
}

impl Watcher {
    pub fn new() -> Watcher {
        Watcher {
            paths: Vec::new(),
        }
    }

    /// Adds a path to the watch list
    pub fn watch_path(&mut self, path: &str) {
        self.paths.push(WatchPath {
            path: PathBuf::from(path),
            last_check: SystemTime::now(),
        });
    }

    /// Checks if any of the files in the watch list have been modified
    pub fn poll(&mut self) {
        for path in self.paths.iter_mut() {
            let last_modified = match fs::metadata(&path.path) {
                Ok(metadata) => if let Ok(last_modified) = metadata.modified() {
                    last_modified
                } else {
                    panic!("std::fs::metadata(...).modified() is not supported on this platform");
                },
                Err(err) => {
                    println!("Failed to access '{}': {}", path.path.to_string_lossy(), err);
                    continue;
                }
            };

            // duration_since returns Ok(_) if last_modified is later than path.last_check,
            // indicating that the file has been modified since the last check
            if let Err(_) = path.last_check.duration_since(last_modified) {
                path.last_check = SystemTime::now();
                println!("File modified: '{}'", path.path.to_string_lossy());
            }
        }
    }
}


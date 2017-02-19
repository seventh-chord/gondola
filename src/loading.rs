
use std::time::SystemTime;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::collections::HashMap;

use shader::{Shader, ShaderPrototype};
use buffer::Vertex;
use texture::{Texture, TextureReference};

pub struct ResourceLoader {
    resources: HashMap<PathBuf, Resource>,
}

#[derive(Debug)]
struct Resource {
    load_time: SystemTime,
    data: ResourceData,
}

#[derive(Debug)]
enum ResourceData {
    Shader(ShaderPrototype),
    Texture(Texture),
}

impl ResourceLoader {
    pub fn new(path: &str) -> io::Result<ResourceLoader> {
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

        let resource = Resource {
            load_time: SystemTime::now(),
            data: ResourceData::Shader(prototype),
        };
        self.resources.insert(PathBuf::from(file), resource);
        Ok(())
    }
    fn load_texture(&mut self, file: &Path) -> io::Result<()> {
        let texture = Texture::load(file)?;

        let resource = Resource {
            load_time: SystemTime::now(),
            data: ResourceData::Texture(texture),
        };
        self.resources.insert(PathBuf::from(file), resource);
        Ok(())
    }

    /// Checks if the asset files have been modified, and reloads them if they have
    pub fn reload_assets(&mut self) {
        for (path, resource) in self.resources.iter_mut() {
            let last_modified = match fs::metadata(&path) {
                Ok(metadata) => if let Ok(last_modified) = metadata.modified() {
                    last_modified
                } else {
                    panic!("std::fs::metadata(...).modified() is not supported on this platform");
                },
                Err(err) => {
                    println!("Failed to access '{}': {}", path.to_string_lossy(), err);
                    continue;
                }
            };

            // duration_since returns Ok(_) if last_modified is later than path.last_check,
            // indicating that the file has been modified since the last check
            if let Err(_) = resource.load_time.duration_since(last_modified) {
                resource.load_time = SystemTime::now();
                match resource.data {
                    ResourceData::Shader(ref shader) => {
                    },
                    ResourceData::Texture(ref mut texture) => {
                        texture.reload();
                    }
                }
                println!("File modified: '{}'", path.to_string_lossy());
            }
        }
    }

    /// Looks for a pre-loaded shader at the given path
    pub fn get_shader<P>(&self, name: P) -> Option<&ShaderPrototype> where P: AsRef<Path> {
        match self.resources.get(name.as_ref()) {
            Some(&Resource { data: ResourceData::Shader(ref prototype), .. } ) => {
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
    pub fn get_texture<P>(&self, name: P) -> Option<TextureReference> where P: AsRef<Path> {
        match self.resources.get(name.as_ref()) {
            Some(&Resource { data: ResourceData::Texture(ref texture), .. } ) => {
                Some(texture.create_reference())
            },
            _ => None
        }
    }
}


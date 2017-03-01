
use gl;
use gl::types::*;
use rusttype;
use rusttype::{Scale, Rect, point, GlyphId, PositionedGlyph};
use rusttype::gpu_cache::*;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::fs::File;
use std::str::Chars;
use cable_math::Vec2;
use cable_math::Mat4;
use texture::*;
use buffer::*;
use shader::*;

const CACHE_SIZE: u32 = 512;

pub struct Font<'a> {
    font: rusttype::Font<'a>,

    cache: Cache,
    cache_texture: Texture,

    buffer: VertexBuffer<FontVert>,
    buffer_data: Vec<FontVert>,

    shader: Shader,
}

impl<'a> Font<'a> {
    pub fn from_file<P>(p: P) -> io::Result<Font<'a>> where P: AsRef<Path> {
        let mut file = File::open(p)?;
        
        let mut data = Vec::new();
        let _bytes_read = file.read_to_end(&mut data)?;

        let font_collection = rusttype::FontCollection::from_bytes(data);
        let font = font_collection.font_at(0).unwrap();

        let cache = Cache::new(CACHE_SIZE, CACHE_SIZE, 0.1, 0.1);

        let mut cache_texture = Texture::new();
        cache_texture.initialize(CACHE_SIZE, CACHE_SIZE, TextureFormat::R_8);

        Ok(Font {
            font: font,
            cache: cache,
            cache_texture: cache_texture,
            buffer: VertexBuffer::with_capacity(PrimitiveMode::Triangles, BufferUsage::DynamicDraw, 200),
            buffer_data: Vec::with_capacity(200),
            shader: build_font_shader(),
        })
    }

    /// Calculates the width, in pixels, of the given string
    pub fn get_width(&self, text: &str, text_size: f32) -> f32 {
        let iter = PosGlyphIter::new(text, &self.font, Scale::uniform(text_size));
        iter.map(|glyph| glyph.unpositioned().h_metrics().advance_width).sum()
    }

    /// Draws the given string. A mutable reference to self is needed as 
    /// glyphs are cached internally. Note that blending should be enabled 
    /// when drawing text.
    pub fn draw(&mut self, mvp: Mat4<f32>, text: &str, text_size: f32) {
        let iter = PosGlyphIter::new(text, &self.font, Scale::uniform(text_size));

        // Push textures to GPU
        {
            for glyph in iter.clone() {
                self.cache.queue_glyph(0, glyph);
            }
            let ref mut tex = self.cache_texture;
            self.cache.cache_queued(|rect, data| {
                tex.load_data_to_region(data,
                                        rect.min.x, rect.min.y,
                                        rect.width(), rect.height());
            }).unwrap();
        }

        // Push render data to GPU
        self.buffer_data.clear();
        for glyph in iter {
            if let Ok(Some((uv, pos))) = self.cache.rect_for(0, &glyph) {
                FontVert::to_buffer(&mut self.buffer_data, pos, uv);
            }
        }
        self.buffer.clear();
        self.buffer.put_at_start(&self.buffer_data);

        // Draw
        self.cache_texture.bind(0);
        self.shader.bind(); // Somehow avoid repeatdly doing this by batching font draw calls with closures similar to how matrix stacks operate
        self.shader.set_uniform("mvp", mvp);
        self.buffer.draw();
    }
}

#[derive(Debug)]
#[repr(C)]
struct FontVert {
    pos: Vec2<f32>,
    uv: Vec2<f32>,
}
// We cannot use the custom derive from within this crate
impl Vertex for FontVert {
    fn bytes_per_vertex() -> usize { 16 }
    fn setup_attrib_pointers() {
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT,
                                    false as GLboolean,
                                    16 as GLsizei, 0 as *const GLvoid);
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT,
                                    false as GLboolean,
                                    16 as GLsizei, 8 as *const GLvoid);
        }
    }
    // Not used, we manualy declare inputs in the shader
    fn gen_shader_input_decl() -> String { String::new() }
}
const VERT_SRC: &'static str = "
    #version 330 core

    layout(location = 0) in vec2 pos;
    layout(location = 1) in vec2 uv;

    out vec2 vert_uv;

    uniform mat4 mvp;

    void main() {
        gl_Position = mvp * vec4(pos, 0.0, 1.0);
        vert_uv = uv;
    }
";
const FRAG_SRC: &'static str = "
    #version 330 core

    in vec2 vert_uv;
    out vec4 color;

    uniform sampler2D tex_sampler;

    void main() {
//        color = texture2D(tex_sampler, vert_uv);
        // Temp. workaround until I implement texture swizeling
        color = vec4(1.0, 1.0, 1.0, texture2D(tex_sampler, vert_uv).r);
    }
";
fn build_font_shader() -> Shader {
    match ShaderPrototype::new_prototype(VERT_SRC, "", FRAG_SRC).build() {
        Ok(shader) => shader,
        Err(err) => {
            println!("{}", err); // Print the error neatly properly
            panic!();
        }
    }
}
impl FontVert {
    fn to_buffer(data: &mut Vec<FontVert>, pos: Rect<i32>, uv: Rect<f32>) {
        let x1 = pos.min.x as f32;
        let x2 = pos.max.x as f32;
        let y1 = -pos.min.y as f32;
        let y2 = -pos.max.y as f32;
        data.push(FontVert {
            pos: Vec2::new(x1, y1),
            uv: Vec2::new(uv.min.x, uv.min.y),
        });
        data.push(FontVert {
            pos: Vec2::new(x2, y1),
            uv: Vec2::new(uv.max.x, uv.min.y),
        });
        data.push(FontVert {
            pos: Vec2::new(x2, y2),
            uv: Vec2::new(uv.max.x, uv.max.y),
        });

        data.push(FontVert {
            pos: Vec2::new(x1, y1),
            uv: Vec2::new(uv.min.x, uv.min.y),
        });
        data.push(FontVert {
            pos: Vec2::new(x2, y2),
            uv: Vec2::new(uv.max.x, uv.max.y),
        });
        data.push(FontVert {
            pos: Vec2::new(x1, y2),
            uv: Vec2::new(uv.min.x, uv.max.y),
        });
    }
}

#[derive(Clone)]
struct PosGlyphIter<'a: 'b, 'b> {
    text: Chars<'b>,

    font: &'b rusttype::Font<'a>,
    scale: Scale,

    caret: Vec2<f32>,
    last_glyph: Option<GlyphId>,
    vertical_advance: f32,
}
impl<'a: 'b, 'b> PosGlyphIter<'a, 'b> {
    fn new(text: &'b str, font: &'a rusttype::Font<'b>, scale: Scale) -> PosGlyphIter<'a, 'b> {
        let v_metrics = font.v_metrics(scale);
        let vertical_advance = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        PosGlyphIter {
            text: text.chars(),

            font: font,
            scale: scale,

            caret: Vec2::new(0.0, 0.0),
            last_glyph: None,
            vertical_advance: vertical_advance,
        }
    }
}
impl<'a: 'b, 'b> Iterator for PosGlyphIter<'a, 'b> {
    type Item = PositionedGlyph<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(c) = self.text.next() {
            // Move to new line
            if c.is_control() {
                if c == '\n' {
                    self.caret.x = 0.0;
                    self.caret.y += self.vertical_advance;
                    self.last_glyph = None; //No kerning after newline
                }
                continue;
            }

            let glyph = if let Some(glyph) = self.font.glyph(c) {
                glyph
            } else {
                continue;
            };

            // Apply kerning
            if let Some(prev) = self.last_glyph.take() {
                self.caret.x += self.font.pair_kerning(self.scale, prev, glyph.id());
            }
            self.last_glyph = Some(glyph.id());

            let glyph = glyph
                .scaled(self.scale)
                .positioned(point(self.caret.x, self.caret.y));
            self.caret.x += glyph.unpositioned().h_metrics().advance_width;
            return Some(glyph);
        }
        None
    }
}


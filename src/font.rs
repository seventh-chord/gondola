
//! This module provides various utilities for rendering text.

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
use texture::*;
use buffer::*;
use shader::*;
use util::graphics;

const CACHE_SIZE: u32 = 512;

/// A font. This struct can be used both to store data in and to draw data from a [`DrawCache`]. 
/// Usually a [`CachedFont`] will be more convenient.
///
/// [`DrawCache`]:  struct.DrawCache.html
/// [`CachedFont`]: struct.CachedFont.html
pub struct Font<'a> {
    font: rusttype::Font<'a>,
    cache: Cache,
    cache_texture: Texture,
    shader: Shader,
}

impl<'a> Font<'a> {
    /// Constructs a new font from the given font file. The file should be in either trutype (`.ttf`) or
    /// opentype (`.otf`) format. See [rusttype documentation](https://docs.rs/rusttype) for a complete 
    /// overview of font support. 
    pub fn from_file<P>(p: P) -> io::Result<Font<'a>> where P: AsRef<Path> {
        let mut file = File::open(p)?;
        
        let mut data = Vec::new();
        let _bytes_read = file.read_to_end(&mut data)?;

        let font_collection = rusttype::FontCollection::from_bytes(data);
        let font = font_collection.font_at(0).unwrap();

        let cache = Cache::new(CACHE_SIZE, CACHE_SIZE, 0.1, 0.1);

        let mut cache_texture = Texture::new();
        cache_texture.initialize(CACHE_SIZE, CACHE_SIZE, TextureFormat::R_8);
        cache_texture.set_swizzle_mask((SwizzleComp::One, SwizzleComp::One, SwizzleComp::One, SwizzleComp::Red));

        Ok(Font {
            font: font,
            cache: cache,
            cache_texture: cache_texture,
            shader: build_font_shader(),
        })
    }

    /// Calculates the width, in pixels, of the given string if it where to be
    /// rendered at the given size. This takes newlines into acount, meaning that
    /// for a multiline string this will return the length of the longest line.
    pub fn get_width(&self, text: &str, text_size: f32) -> f32 {
        let iter = PosGlyphIter::new(text, &self.font, Scale::uniform(text_size), Vec2::zero());
        iter.map(|glyph| glyph.unpositioned().h_metrics().advance_width).sum()
    }

    /// Writes data needed to render the given text into the given render cache. Multiple pieces of
    /// text can be written into the render cache before rendering it. This allows for efficient
    /// rendering of large sets of text.
    pub fn cache(&mut self, draw_cache: &mut DrawCache, text: &str, text_size: f32, offset: Vec2<f32>) {
        let iter = PosGlyphIter::new(text, &self.font, Scale::uniform(text_size), offset);

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
        for glyph in iter {
            if let Ok(Some((uv, pos))) = self.cache.rect_for(0, &glyph) {
                FontVert::to_buffer(&mut draw_cache.buffer_data, pos, uv);
            }
        }
    }

    /// Draws teh data stored in the given draw cache. Note that you should call
    /// [`DrawCache::update_vbo`] before drawing a cache, and you probably want to
    /// call [`DrawCache::clear`] afterwards.
    ///
    /// [`DrawCache::update_vbo`]:  struct.DrawCache.html#method.update_vbo
    /// [`DrawCache::clear`]:       struct.DrawCache.html#method.clear
    pub fn draw_cache(&mut self, cache: &DrawCache) {
        graphics::set_blending(Some(graphics::BlendSettings::default()));
        self.shader.bind();
        self.cache_texture.bind(0);
        cache.buffer.draw();
        graphics::set_blending(None);
    }
}

/// A draw cache contains the raw data that is sent to the GPU when rendering font. This struct is
/// used to temporarily store data during cached rendering. Large amounts of text can be written to
/// the cache and then drawn with a single rendercall, allowing for more efficient rendering.
///
/// The cache can be filled with [`Font::cache`], and its contents can be drawn with [`Font::draw_cache`].
/// Note that the internal vertex buffer needs to be updated with [`DrawCache::update_vbo`] before
/// rendering. You probably also want to clear the buffer with [`DrawCache::clear`] after rendering,
/// otherwise the same data will be redrawn in the next frame.
///
/// Note that a cache should only ever be used with a single [`Font`] per call to [`Font::draw_cache`]. 
/// This is because a `DrawCache` has no knowledge of which font provided the data stored in it. Failing 
/// to comply to this rule will lead to garbled text beeing rendered.
///
/// If you do not need the `Font` - `DrawCache` separation you might want to concider using
/// [`CachedFont`] instead, as it provides a draw cache and a font in a
/// single cohesive package.
///
/// [`Font`]:                   struct.Font.html
/// [`Font::cache`]:            struct.Font.html#method.cache
/// [`Font::draw_cache`]:       struct.Font.html#method.draw_cache
/// [`DrawCache::update_vbo`]:  struct.DrawCache.html#method.update_vbo
/// [`DrawCache::clear`]:       struct.DrawCache.html#method.clear
/// [`CachedFont`]:             struct.CachedFont.html
pub struct DrawCache {
    buffer: VertexBuffer<FontVert>,
    buffer_data: Vec<FontVert>,
}

impl DrawCache {
    /// Constructs a new, empty, draw cache
    pub fn new() -> DrawCache {
        DrawCache {
            buffer: VertexBuffer::with_capacity(PrimitiveMode::Triangles, BufferUsage::DynamicDraw, 500),
            buffer_data: Vec::with_capacity(500),
        }
    }

    /// When drawing to this cache, data is by default stored on the CPU side. This method moves
    /// data over to the GPU.
    pub fn update_vbo(&mut self) {
        self.buffer.clear();
        self.buffer.put(0, &self.buffer_data);
    }

    /// Removes all data from this cache. Note that this does not call any expensive operations,
    /// it simply sets the size of the internal buffers to 0.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.buffer_data.clear();
    }
}

/// A thin wrapper around a [`Font`] coupled with a [`DrawCache`]. This is intended for simple text
/// rendering, and is probably adequate for most usecases.
///
/// [`Font`]:       struct.Font.html
/// [`DrawCache`]:  struct.DrawCache.html
pub struct CachedFont<'a> {
    font: Font<'a>,
    draw_cache: DrawCache,
}

impl<'a> CachedFont<'a> {
    /// Constructs a new cached font from the given font file. The file should be in either trutype (`.ttf`)
    /// or opentype (`.otf`) format. See [rusttype documentation](https://docs.rs/rusttype) for a complete 
    /// overview of font support. 
    pub fn from_file<P>(p: P) -> io::Result<CachedFont<'a>> where P: AsRef<Path> {
        Ok(CachedFont {
            font: Font::from_file(p)?,
            draw_cache: DrawCache::new(),
        })
    }

    /// Adds the given piece of text to the internal draw cache. Cached text can be drawn with 
    /// [`CachedFont::draw`](struct.CachedFont.html#method.draw)
    pub fn cache(&mut self, text: &str, size: f32, pos: Vec2<f32>) {
        self.font.cache(&mut self.draw_cache, text, size, pos);
    }

    /// Draws all text in the internal cache and then clears the cache
    pub fn draw(&mut self) {
        self.draw_cache.update_vbo();
        self.font.draw_cache(&self.draw_cache);
        self.draw_cache.clear();
    }

    pub fn font(&self) -> &Font<'a> { &self.font }
    pub fn font_mut(&mut self) -> &mut Font<'a> { &mut self.font }
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

    // Matrix block is inserted automatically

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
        color = texture2D(tex_sampler, vert_uv);
    }
";
fn build_font_shader() -> Shader {
    let mut proto = ShaderPrototype::new_prototype(VERT_SRC, "", FRAG_SRC);
    proto.bind_to_matrix_storage();
    match proto.build() {
        Ok(shader) => shader,
        Err(err) => {
            println!("{}", err); // Print the error properly
            panic!();
        }
    }
}
impl FontVert {
    fn to_buffer(data: &mut Vec<FontVert>, pos: Rect<i32>, uv: Rect<f32>) {
        let x1 = pos.min.x as f32;
        let x2 = pos.max.x as f32;
        let y1 = pos.min.y as f32;
        let y2 = pos.max.y as f32;
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

    offset: Vec2<f32>,
    caret: Vec2<f32>,
    last_glyph: Option<GlyphId>,
    vertical_advance: f32,
}
impl<'a: 'b, 'b> PosGlyphIter<'a, 'b> {
    fn new(text: &'b str, font: &'a rusttype::Font<'b>, scale: Scale, offset: Vec2<f32>) -> PosGlyphIter<'a, 'b> {
        let v_metrics = font.v_metrics(scale);
        let vertical_advance = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        PosGlyphIter {
            text: text.chars(),

            font: font,
            scale: scale,

            caret: offset,
            offset: offset,
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
                    self.caret.x = self.offset.x;
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


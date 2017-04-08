
//! This module provides various utilities for rendering text.

// Note to self: There is a problem with the current font rendering system. When storing data
// in a draw cache, we write data to the cache texture. If the cache texture is to small we will
// end up overwriting the original data in the texture with new data before rendering. If this
// happens we can probably solve the problem by simply increasing the cache texture size.

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
use std::ops::Range;
use cable_math::Vec2;
use texture::{Texture, SwizzleComp, TextureFormat};
use buffer::{Vertex, VertexBuffer, BufferUsage, PrimitiveMode};
use shader::{ShaderPrototype, Shader};
use color::Color;
use util::graphics;

const CACHE_TEX_SIZE: u32 = 1024; // More than 99% of GPUs support this texture size: http://feedback.wildfiregames.com/report/opengl/feature/GL_MAX_TEXTURE_SIZE
const VERTS_PER_CHAR: usize = 6;
const CACHE_SIZE: usize = 500;

/// A font. This struct can be used both to store data in and to draw data from a [`DrawCache`]. 
/// Usually a [`CachedFont`] will be more convenient.
///
/// [`DrawCache`]:  struct.DrawCache.html
/// [`CachedFont`]: struct.CachedFont.html
pub struct Font {
    font: rusttype::Font<'static>,
    cache: Cache,
    cache_texture: Texture,
    shader: Shader,
}

impl Font {
    /// Constructs a new font from the given font file. The file should be in either trutype (`.ttf`) or
    /// opentype (`.otf`) format. See [rusttype documentation](https://docs.rs/rusttype) for a complete 
    /// overview of font support. 
    pub fn from_file<P>(p: P) -> io::Result<Font> where P: AsRef<Path> {
        let mut file = File::open(p)?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let font_collection = rusttype::FontCollection::from_bytes(data);
        let font = font_collection.font_at(0).unwrap();

        Ok(Font::with_rusttype_font(font))
    }
    fn with_rusttype_font(font: rusttype::Font<'static>) -> Font {
        let cache = Cache::new(CACHE_TEX_SIZE, CACHE_TEX_SIZE, 0.1, 0.1);

        let mut cache_texture = Texture::new();
        cache_texture.initialize(CACHE_TEX_SIZE, CACHE_TEX_SIZE, TextureFormat::R_8);
        cache_texture.set_swizzle_mask((SwizzleComp::One, SwizzleComp::One, SwizzleComp::One, SwizzleComp::Red));

        Font {
            font: font,
            cache: cache,
            cache_texture: cache_texture,
            shader: build_shader(),
        }
    }

    /// Calculates the width, in pixels, of the given string if it where to be
    /// rendered at the given size. This takes newlines into acount, meaning that
    /// for a multiline string this will return the length of the longest line.
    pub fn width(&self, text: &str, text_size: f32) -> f32 {
        let iter = PlacementIter::new(text, &self.font, Scale::uniform(text_size), Vec2::zero());
        let mut max_width = 0.0;
        for PlacementInfo { caret, .. } in iter {
            max_width = f32::max(caret.x, max_width);
        }
        max_width
    }

    /// Calculates which region of the given piece of text will be visible in a
    /// viewport with the given width. `focus` specifies which codepoint of the string
    /// should be in the center of the viewport. For example, if `focus` is set to
    /// `text.len() - 1` this function will find a range of characters starting from
    /// the end which will fit into the given width.
    ///
    /// Panics if `focus` is not a valid index to `text`. A valid index is within
    /// the length of the text and on a character boundary. Keep in mind that
    /// `focus` is a byte index, not a char index.  Returns a range that can be used 
    /// to take a valid slice of `text`, and the draw space coordinate of where the
    /// caret should be drawn if this text slice is drawn.
    ///
    /// This function has not been tested with multiline strings.
    pub fn visible_area(&self, text: &str, text_size: f32, width: f32, focus: usize) -> (Range<usize>, f32) {
        if focus > text.len() && text.is_char_boundary(focus) { 
            panic!("`focus` is not a valid index (focus = {})", focus);
        }

        let mut focus_pos = 0.0;
        let mut text_width = 0.0; 
        let iter = PlacementIter::new(text, &self.font, Scale::uniform(text_size), Vec2::zero());

        // Find the location within the text, in draw space coordinates, which should be in focus
        for PlacementInfo { caret, str_index, .. } in iter.clone() {
            if str_index == focus {
                focus_pos = caret.x;
            }
            if caret.x > text_width { text_width = caret.x; }
        }

        // Calculate the start and end, in draw space coordinates, of the visible region
        let (start, end) = {
            let start = f32::min(focus_pos + width/2.0, text_width) - width;
            if start < 0.0 {
                (0.0, f32::min(width, text_width))
            } else {
                (start, start + width)
            }
        };

        let mut range = 0..text.len();
        let mut caret_pos = 0.0;
        let mut actual_start = 0.0;

        // Find the byte indices of the start and end coordinates
        for PlacementInfo { caret, str_index, .. } in iter {
            if caret.x < start {
                range.start = str_index;
                actual_start = caret.x;
            }
            if str_index == focus {
                caret_pos = caret.x - actual_start;
            }
            if caret.x > actual_start + (end - start) {
                range.end = str_index;
                break;
            }
        }

        (range, caret_pos)
    }

    /// Finds the index of the character that would be under the cursor if the cursor is at the
    /// given x-offset (`pos`) from the start of where the text is drawn. The returned index is
    /// a byte index to the given piece of text.
    pub fn hovered_char(&self, text: &str, text_size: f32, pos: f32) -> Option<usize> {
        let iter = PlacementIter::new(text, &self.font, Scale::uniform(text_size), Vec2::zero());
        for PlacementInfo { caret, glyph, str_index } in iter {
            let width = glyph.unpositioned().h_metrics().advance_width;
            if caret.x + width/2.0 >= pos {
                return Some(str_index);
            }
        }
        None
    }

    /// Retrieves the total height of a line drawn with this font at the given size. This is the
    /// sum of the max ascent, the max descent and the line gap.
    pub fn line_height(&self, text_size: f32) -> f32 {
        let v_metrics = self.font.v_metrics(Scale::uniform(text_size));
        v_metrics.ascent - v_metrics.descent + v_metrics.line_gap
    }

    /// Retrieves the max ascent of a line drawn with this font at the given size. Note that this
    /// number is typically positive
    pub fn ascent(&self, text_size: f32) -> f32 {
        let v_metrics = self.font.v_metrics(Scale::uniform(text_size));
        v_metrics.ascent
    }
    /// Retrieves the min descent of a line drawn with this font at the given size. Note that this
    /// number is typically negative
    pub fn descent(&self, text_size: f32) -> f32 {
        let v_metrics = self.font.v_metrics(Scale::uniform(text_size));
        v_metrics.descent
    }
    /// Retrieves the line gap that should be used with this font at the given size.
    pub fn line_gap(&self, text_size: f32) -> f32 {
        let v_metrics = self.font.v_metrics(Scale::uniform(text_size));
        v_metrics.line_gap
    }

    /// Writes data needed to render the given text into the given render cache. Multiple pieces of
    /// text can be written into the render cache before rendering it. This allows for efficient
    /// rendering of large sets of text.
    pub fn cache(&mut self, draw_cache: &mut DrawCache, text: &str, text_size: f32, offset: Vec2<f32>, color: Color) {
        let iter = PlacementIter::new(text, &self.font, Scale::uniform(text_size), offset);
        for PlacementInfo { glyph, .. } in iter {
            if let Ok(Some((uv, pos))) = self.cache.rect_for(0, &glyph) {
                FontVert::to_buffer(&mut draw_cache.buffer_data, pos, uv, color);
            }

            self.cache.queue_glyph(0, glyph);
        }

        let ref mut tex = self.cache_texture;
        self.cache.cache_queued(|rect, data| {
            tex.load_data_to_region(data,
                                    rect.min.x, rect.min.y,
                                    rect.width(), rect.height());
        }).unwrap();
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

impl Clone for Font {
    /// Produces a copy of this font. Note that this creates a new internal glyph cache
    fn clone(&self) -> Font {
        // Cloning a rusttype font is cheap as data is internally stored in a
        // `Arc<Box<&[u8]>>`, which is cheap to clone.
        Font::with_rusttype_font(self.font.clone())
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
        let vertices = VERTS_PER_CHAR * CACHE_SIZE;
        DrawCache {
            buffer: VertexBuffer::with_capacity(PrimitiveMode::Triangles, BufferUsage::DynamicDraw, vertices),
            buffer_data: Vec::with_capacity(vertices),
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
/// # Example
/// ```rust,ignore
/// use gondola::font::CachedFont;
/// use cable_math::Vec2;
///
/// let mut font = CachedFont::from_file("assets/comic_sans.ttf").unwrap();
///
/// loop {
///     // Main logic goes here ...
///     font.cache("Hello world\nTesting", 14.0, Vec2::new(50.0, 50.0));
///     font.cache("ax² + bx + c = 0", 14.0, Vec2::new(50.0, 100.0));
///     font.draw();
/// }
/// ```
///
/// [`Font`]:       struct.Font.html
/// [`DrawCache`]:  struct.DrawCache.html
pub struct CachedFont {
    font: Font,
    draw_cache: DrawCache,
}

impl CachedFont {
    /// Constructs a new cached font from the given font file. The file should be in either trutype (`.ttf`)
    /// or opentype (`.otf`) format. See [rusttype documentation](https://docs.rs/rusttype) for a complete 
    /// overview of font support. 
    pub fn from_file<P>(p: P) -> io::Result<CachedFont> where P: AsRef<Path> {
        Ok(CachedFont {
            font: Font::from_file(p)?,
            draw_cache: DrawCache::new(),
        })
    }

    /// Wrapps the given font in a cached font
    pub fn from_font(font: Font) -> CachedFont {
        CachedFont {
            font: font,
            draw_cache: DrawCache::new(),
        }
    }

    /// Adds the given piece of text to the internal draw cache. Cached text can be drawn with 
    /// [`draw`](struct.CachedFont.html#method.draw). Usually you want to cache all text you want
    /// to draw in a given frame and then draw it all in a single call.
    pub fn cache(&mut self, text: &str, size: f32, pos: Vec2<f32>, color: Color) {
        self.font.cache(&mut self.draw_cache, text, size, pos, color);
    }

    /// Draws all text in the internal cache and then clears the cache
    pub fn draw(&mut self) {
        self.draw_cache.update_vbo();
        self.font.draw_cache(&self.draw_cache);
        self.draw_cache.clear();
    }

    pub fn font(&self) -> &Font { &self.font }
    pub fn font_mut(&mut self) -> &mut Font { &mut self.font }
}
#[derive(Debug)]
#[repr(C)]
struct FontVert {
    pos: Vec2<f32>,
    uv: Vec2<f32>,
    color: Color,
}
// We cannot use the custom derive from within this crate
impl Vertex for FontVert {
    fn bytes_per_vertex() -> usize { ::std::mem::size_of::<FontVert>() }
    fn setup_attrib_pointers() {
        let stride = Self::bytes_per_vertex();
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, 0 as *const GLvoid);
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, 8 as *const GLvoid);
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 4, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, 16 as *const GLvoid);
        }
    }
    // Not used, we manualy declare inputs in the shader
    fn gen_shader_input_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_outputs(_name_prefix: &str) -> Vec<String> { Vec::new() }
}
const VERT_SRC: &'static str = "
    #version 330 core

    layout(location = 0) in vec2 pos;
    layout(location = 1) in vec2 uv;
    layout(location = 2) in vec4 color;

    out vec2 vert_uv;
    out vec4 vert_color;

    // Matrix block is inserted automatically

    void main() {
        gl_Position = mvp * vec4(pos, 0.0, 1.0);
        vert_uv = uv;
        vert_color = color;
    }
";
const FRAG_SRC: &'static str = "
    #version 330 core

    in vec2 vert_uv;
    in vec4 vert_color;
    out vec4 color;

    uniform sampler2D tex_sampler;

    void main() {
        color = vert_color * texture2D(tex_sampler, vert_uv);
    }
";
fn build_shader() -> Shader {
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
    fn to_buffer(data: &mut Vec<FontVert>, pos: Rect<i32>, uv: Rect<f32>, color: Color,) {
        let x1 = pos.min.x as f32;
        let x2 = pos.max.x as f32;
        let y1 = pos.min.y as f32;
        let y2 = pos.max.y as f32;
        data.push(FontVert {
            pos: Vec2::new(x1, y1),
            uv: Vec2::new(uv.min.x, uv.min.y),
            color: color,
        });
        data.push(FontVert {
            pos: Vec2::new(x2, y1),
            uv: Vec2::new(uv.max.x, uv.min.y),
            color: color,
        });
        data.push(FontVert {
            pos: Vec2::new(x2, y2),
            uv: Vec2::new(uv.max.x, uv.max.y),
            color: color,
        });

        data.push(FontVert {
            pos: Vec2::new(x1, y1),
            uv: Vec2::new(uv.min.x, uv.min.y),
            color: color,
        });
        data.push(FontVert {
            pos: Vec2::new(x2, y2),
            uv: Vec2::new(uv.max.x, uv.max.y),
            color: color,
        });
        data.push(FontVert {
            pos: Vec2::new(x1, y2),
            uv: Vec2::new(uv.min.x, uv.max.y),
            color: color,
        });
    }
}

#[derive(Clone)]
struct PlacementIter<'a> {
    text: Chars<'a>,
    str_index: usize,

    font: &'a rusttype::Font<'a>,
    scale: Scale,

    offset: Vec2<f32>,
    caret: Vec2<f32>,
    last_glyph: Option<GlyphId>,
    vertical_advance: f32,
}
struct PlacementInfo<'a> {
    glyph: PositionedGlyph<'a>, 
    caret: Vec2<f32>,
    str_index: usize,
}

impl<'a> PlacementIter<'a> {
    fn new(text: &'a str, font: &'a rusttype::Font, scale: Scale, offset: Vec2<f32>) -> PlacementIter<'a> {
        let v_metrics = font.v_metrics(scale);
        let vertical_advance = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        PlacementIter {
            text: text.chars(),
            str_index: 0,

            font: font,
            scale: scale,

            offset: offset,
            caret: offset,
            last_glyph: None,
            vertical_advance: vertical_advance,
        }
    }
}

impl<'a> Iterator for PlacementIter<'a> {
    type Item = PlacementInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(c) = self.text.next() {
            self.str_index += c.len_utf8();

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

            return Some(PlacementInfo {
                glyph: glyph,
                caret: self.caret,
                str_index: self.str_index,
            });
        }
        None
    } 
}


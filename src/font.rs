
//! This module provides various utilities for rendering text.

// Note to self: There is a problem with the current font rendering system. When storing data
// in a draw cache, we write data to the cache texture. If the cache texture is to small we will
// end up overwriting the original data in the texture with new data before rendering. If this
// happens we can probably solve the problem by simply increasing the cache texture size.

use gl;
use gl::types::*;
use rusttype;
use rusttype::{Scale, point, GlyphId, PositionedGlyph};
use rusttype::gpu_cache::*;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::fs::File;
use std::str::Chars;
use std::ops::Range;

use cable_math::Vec2;

use texture::{Texture, SwizzleComp, TextureFormat};
use buffer::Vertex;
use color::Color;

const CACHE_TEX_SIZE: u32 = 1024; // More than 99% of GPUs support this texture size: http://feedback.wildfiregames.com/report/opengl/feature/GL_MAX_TEXTURE_SIZE

// There might be some official sepc for how tabs should work. Note that this is multiplied by the
// current font size.
const TAB_WIDTH: f32 = 1.5;

/// A single font style. This is not used directly for text rendering, but rather specifies how
/// text should be layed out according to a given font. It also provides rasterized glyphs that are
/// needed when drawing text.
pub struct Font {
    font: rusttype::Font<'static>,
    gpu_cache: Cache,
    cache_texture: Texture,
}

impl Font {
    /// Constructs a new font from the given font file. The file should be in either trutype
    /// (`.ttf`) or opentype (`.otf`) format. See [rusttype documentation][1] for a complete 
    /// overview of font support. 
    /// 
    /// [1]: https://docs.rs/rusttype
    pub fn from_file<P>(p: P) -> io::Result<Font> where P: AsRef<Path> {
        let mut file = File::open(p)?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let font_collection = rusttype::FontCollection::from_bytes(data);
        let font = font_collection.font_at(0).unwrap();

        Ok(Font::with_rusttype_font(font))
    }

    /// Constructs a font from raw data bytes. This can be used in conjunction with the
    /// `include_bytes!(...)` macro. This function expects fonts in the same format as
    /// `Font::from_file`.
    pub fn from_bytes(bytes: &'static [u8]) -> Font {
        let font_collection = rusttype::FontCollection::from_bytes(bytes);
        let font = font_collection.font_at(0).unwrap();

        Font::with_rusttype_font(font)
    }

    fn with_rusttype_font(font: rusttype::Font<'static>) -> Font {
        let gpu_cache = Cache::new(CACHE_TEX_SIZE, CACHE_TEX_SIZE, 0.5, 0.5);

        let mut cache_texture = Texture::new();
        cache_texture.initialize(CACHE_TEX_SIZE, CACHE_TEX_SIZE, TextureFormat::R_8);
        cache_texture.set_swizzle_mask((SwizzleComp::One, SwizzleComp::One, SwizzleComp::One, SwizzleComp::Red));

        Font { font, gpu_cache, cache_texture }
    }

    /// Calculates the width in pixels of the given string if it where to be rendered at the given
    /// size. This takes newlines into acount. 
    pub fn width(&self, text: &str, text_size: f32) -> f32 {
        let mut prev_glyph: Option<GlyphId> = None; 
        let mut caret = Vec2::zero();
        let mut max_x = 0.0;

        let scale = Scale::uniform(text_size);

        for c in text.chars() {
            let glyph = if let Some(glyph) = self.font.glyph(c) {
                glyph
            } else {
                continue;
            }; 

            if c.is_control() {
                if c == '\n' {
                    caret.x = 0.0;
                }
                // Align to next tab stop
                if c == '\t' {
                    let tab_width = TAB_WIDTH*text_size;
                    caret.x /= tab_width;
                    caret.x = (caret.x + 1.0).round();
                    caret.x *= tab_width;
                }
                continue;
            }

            // Apply kerning
            if let Some(prev) = prev_glyph.take() {
                caret.x += self.font.pair_kerning(scale, prev, glyph.id());
            }
            prev_glyph = Some(glyph.id());

            let glyph = glyph.scaled(scale);
            caret.x += glyph.h_metrics().advance_width;

            if caret.x > max_x { max_x = caret.x } 
        }

        max_x
    }

    /// Calculates the dimensions, in pixels, of the given string if it where to be rendered at the
    /// given size. This takes newlines into acount. 
    /// Returns the size of the string, in addition to the ascent of the first line. If the text is
    /// offset downwards by this amount the top of the text will be at the previous baseline.
    pub fn dimensions(&self, text: &str, text_size: f32, wrap_width: Option<f32>) -> (Vec2<f32>, f32) {
        let mut prev_glyph: Option<GlyphId> = None; 
        let mut first_line = true;
        let mut first_ascent = 0.0;
        let mut caret = Vec2::zero();
        let mut max_x = 0.0;

        let scale = Scale::uniform(text_size);
        let v_metrics = self.font.v_metrics(scale);
        let vertical_advance = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap; 

        for c in text.chars() {
            let glyph = if let Some(glyph) = self.font.glyph(c) {
                glyph
            } else {
                continue;
            };

            // Move to new line
            if c.is_control() {
                if c == '\n' {
                    first_line = false;
                    max_x = f32::max(max_x, caret.x);
                    caret.x = 0.0;
                    caret.y += vertical_advance;
                    prev_glyph = None; //No kerning after newline
                }
                // Align to next tab stop
                if c == '\t' {
                    let tab_width = TAB_WIDTH*text_size;
                    caret.x /= tab_width;
                    caret.x = (caret.x + 1.0).round();
                    caret.x *= tab_width;
                }
                continue;
            }

            // Apply kerning
            if let Some(prev) = prev_glyph.take() {
                caret.x += self.font.pair_kerning(scale, prev, glyph.id());
            }
            prev_glyph = Some(glyph.id());

            let glyph = glyph.scaled(scale);
            caret.x += glyph.h_metrics().advance_width;

            // Wrap if line is to long
            if let Some(width) = wrap_width {
                if caret.x > width {
                    max_x = f32::max(max_x, caret.x);
                    caret.x = 0.0;
                    caret.y += vertical_advance;
                    prev_glyph = None;
                }
            }

            if first_line {
                if let Some(bounding) = glyph.exact_bounding_box() {
                    first_ascent = f32::max(first_ascent, -bounding.min.y);
                }
            }
        }

        max_x = f32::max(max_x, caret.x);
        if let Some(width) = wrap_width {
            max_x = f32::min(max_x, width);
        }

        (Vec2::new(max_x, caret.y + first_ascent), first_ascent)
    }

    /// Calculates the dimensions of a single line of text. Any newlines in the given string
    /// are ignored.
    pub fn line_dimensions(&self, text: &str, text_size: f32) -> LineDimensions {
        let mut prev_glyph: Option<GlyphId> = None;
        let mut dimensions = LineDimensions::default();

        let scale = Scale::uniform(text_size);

        for c in text.chars() {
            let glyph = if let Some(glyph) = self.font.glyph(c) {
                glyph
            } else {
                continue;
            };

            // Apply kerning
            if let Some(prev) = prev_glyph.take() {
                dimensions.width += self.font.pair_kerning(scale, prev, glyph.id());
            }
            prev_glyph = Some(glyph.id());

            let glyph = glyph.scaled(scale);
            dimensions.width += glyph.h_metrics().advance_width;

            if let Some(bounding) = glyph.exact_bounding_box() {
                dimensions.descent = f32::max(dimensions.descent, bounding.max.y);
                dimensions.ascent = f32::min(dimensions.ascent, bounding.min.y);
            }
        }

        dimensions
    }

    /// Calculates which region of the given piece of text will be visible in a viewport with the
    /// given width. `focus` specifies which codepoint of the string should be in the center of the
    /// viewport. For example, if `focus` is set to `text.len() - 1` this function will find a
    /// range of characters starting from the end which will fit into the given width.
    ///
    /// Panics if `focus` is not a valid index to `text`. A valid index is within the length of the
    /// text and on a character boundary. Keep in mind that `focus` is a byte index, not a char
    /// index.  Returns a range that can be used to take a valid slice of `text`, and the draw
    /// space coordinate of where the caret should be drawn if this text slice is drawn.
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

    /// Calculates the index of the last completly visible character, and the width of the total
    /// visible text (Not including partially visible characters) if the text where to be drawn in
    /// a constrained space.
    ///
    /// Not tested for multiline strings!
    pub fn cutoff(&self, text: &str, text_size: f32, space: f32) -> (usize, f32) {
        let mut index = 0;
        let mut width = 0.0;

        let mut prev = (0, 0.0);

        let iter = PlacementIter::new(text, &self.font, Scale::uniform(text_size), Vec2::zero());
        for PlacementInfo { caret, str_index, .. } in iter.clone() {
            if caret.x > space {
                break;
            } else {
                prev = (index, width);
                index = str_index;
                width = caret.x;
            }
        }

        prev
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

    /// Retrieves height metrics for this font at the given size. This includes the max ascent,
    /// descent and the recommended line gap.
    pub fn height_metrics(&self, text_size: f32) -> HeightMetrics {
        let v_metrics = self.font.v_metrics(Scale::uniform(text_size));
        HeightMetrics {
            ascent: v_metrics.ascent,
            descent: v_metrics.descent,
            line_gap: v_metrics.line_gap,
        }
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

    /// Retrieves the texture in which glyphs for this font are cached. This texture can change
    /// from frame to frame.
    pub fn texture(&self) -> &Texture {
        &self.cache_texture
    }

    /// Writes data needed to render the given text into the given buffer. Multiple pieces of
    /// text can be written into a single buffer before rendering it. This allows for efficient
    /// rendering of large sets of text.
    ///
    /// Returns the number of vertices that where added to the buffer. 
    pub fn cache<T>(
        &mut self,
        buf:        &mut Vec<T>,
        text:       &str,
        text_size:  f32,
        scale:      f32,
        offset:     Vec2<f32>,
        wrap_width: Option<f32>,
        color: Color,
    ) -> usize
        where T: AsFontVert,
    {
        let mut iter = PlacementIter::new(text, &self.font, Scale::uniform(text_size), offset);
        iter.wrap_width = wrap_width;

        // Cache stuff on gpu
        for PlacementInfo { ref glyph, .. } in iter.clone() {
            self.gpu_cache.queue_glyph(0, glyph.clone());
        }
        let ref mut tex = self.cache_texture;
        self.gpu_cache.cache_queued(|rect, data| {
            tex.load_data_to_region(
                data,
                rect.min.x, rect.min.y,
                rect.width(), rect.height()
            );
        }).unwrap();

        // Output vertices
        let mut vertices = 0;
        for PlacementInfo { ref glyph, .. } in iter {
            if let Ok(Some((uv, pos))) = self.gpu_cache.rect_for(0, glyph) {
                let x1 = (pos.min.x as f32 - offset.x)*scale + offset.x;
                let x2 = (pos.max.x as f32 - offset.x)*scale + offset.x;
                let y1 = (pos.min.y as f32 - offset.y)*scale + offset.y;
                let y2 = (pos.max.y as f32 - offset.y)*scale + offset.y;

                buf.push(T::gen(Vec2::new(x1, y1), Vec2::new(uv.min.x, uv.min.y), color));
                buf.push(T::gen(Vec2::new(x2, y1), Vec2::new(uv.max.x, uv.min.y), color));
                buf.push(T::gen(Vec2::new(x2, y2), Vec2::new(uv.max.x, uv.max.y), color));

                buf.push(T::gen(Vec2::new(x1, y1), Vec2::new(uv.min.x, uv.min.y), color));
                buf.push(T::gen(Vec2::new(x2, y2), Vec2::new(uv.max.x, uv.max.y), color));
                buf.push(T::gen(Vec2::new(x1, y2), Vec2::new(uv.min.x, uv.max.y), color));

                vertices += 6;
            }
        }

        vertices 
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

/// This trait can be used to allow drawing fonts into buffers with a custom vertex type.
/// This is primarily used in [`Font::cache`].
///
/// [`Font::cache`]: struct.Font.html#method.cache
pub trait AsFontVert: Vertex {
    fn gen(pos: Vec2<f32>, uv: Vec2<f32>, color: Color) -> Self;
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

impl AsFontVert for FontVert {
    fn gen(pos: Vec2<f32>, uv: Vec2<f32>, color: Color) -> FontVert {
        FontVert { pos, uv, color }
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
    prev_glyph: Option<GlyphId>,
    vertical_advance: f32,

    wrap_width: Option<f32>,
}
struct PlacementInfo<'a> {
    glyph: PositionedGlyph<'a>, 
    caret: Vec2<f32>,
    str_index: usize,
}

impl<'a> PlacementIter<'a> {
    fn new(
        text: &'a str,
        font: &'a rusttype::Font,
        scale: Scale,
        offset: Vec2<f32>
    ) -> PlacementIter<'a> 
    {
        let v_metrics = font.v_metrics(scale);
        let vertical_advance = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        PlacementIter {
            text: text.chars(),
            str_index: 0,

            font: font,
            scale: scale,

            offset: offset,
            caret: offset,
            prev_glyph: None,
            vertical_advance: vertical_advance,

            wrap_width: None,
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
                    self.prev_glyph = None; //No kerning after newline
                }
                // Align to next tab stop
                if c == '\t' {
                    let tab_width = TAB_WIDTH*self.scale.x;

                    let mut x = self.caret.x;
                    x = (x - self.offset.x)/tab_width;
                    x = (x + 1.0).round();
                    x = x*tab_width + self.offset.x;
                    self.caret.x = x;
                }
                continue;
            }

            let glyph = if let Some(glyph) = self.font.glyph(c) {
                glyph
            } else {
                continue;
            };

            let mut advance = 0.0;

            // Apply kerning
            if let Some(prev) = self.prev_glyph.take() {
                advance += self.font.pair_kerning(self.scale, prev, glyph.id());
            }
            self.prev_glyph = Some(glyph.id());

            let glyph = glyph.scaled(self.scale);
            advance += glyph.h_metrics().advance_width;

            self.caret.x += advance;

            if let Some(width) = self.wrap_width {
                if self.caret.x + advance > self.offset.x + width {
                    self.caret.x = self.offset.x + advance;
                    self.caret.y += self.vertical_advance;
                }
            }

            let glyph = glyph.positioned(point(self.caret.x - advance, self.caret.y));


            return Some(PlacementInfo {
                glyph: glyph,
                caret: self.caret,
                str_index: self.str_index,
            });
        }
        None
    } 
}

/// The exact dimensions of a single line of text.
#[derive(Debug, Copy, Clone, Default)]
pub struct LineDimensions {
    /// The distance from the baseline to the top of the highest-reaching glyph. This is
    /// usually negative, as negative y (lower line numbers) is up on paper.
    pub ascent: f32,
    /// The distance from the baseline to the bottom of the lowest-reaching glyph. This is
    /// usually positive, as positive y (higher line numbers) is down on paper.
    pub descent: f32,
    pub width: f32,
}

impl LineDimensions {
    /// The total height of this line. `descent` - `ascent`.
    pub fn height(&self) -> f32 {
        self.descent - self.ascent
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct HeightMetrics {
    /// The distance from the baseline to the top of the highest-reaching glyph. This is
    /// usually negative, as negative y (lower line numbers) is up on paper.
    pub ascent: f32,
    /// The distance from the baseline to the bottom of the lowest-reaching glyph. This is
    /// usually positive, as positive y (higher line numbers) is down on paper.
    pub descent: f32,
    /// The gap there should between the top of one line and the bottom of the next line. This is
    /// just a guideline.
    pub line_gap: f32,
}

impl HeightMetrics {
    /// The total height of this line. `descent` - `ascent`.
    pub fn height(&self) -> f32 {
        self.ascent - self.descent + self.line_gap
    }
}

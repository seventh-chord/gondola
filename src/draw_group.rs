
//! Utilities for graphics

use std::f32;
use std::io;
use std::path::Path;
use std::hash::Hash;
use std::collections::HashMap;

use cable_math::{Vec2, Mat4};

use Color;
use graphics; 
use Region;
use shader::{ShaderPrototype, Shader};
use texture::{Texture, TextureFormat};
use buffer::{AttribBinding, Vertex, PrimitiveMode, BufferUsage, VertexBuffer};
use font::{BitmapFont, TruetypeFont};

// This could be a const generic in the future, but that is not implemented in rust yet
pub const LAYER_COUNT: usize = 2;

/// Batches drawcalls for 2d primitive and text rendering. Things can be rendered with transparency
/// and in various layers. 
///
/// `TruetypeFontKey` is some type used to identify truetype fonts. Typically you would want to 
/// use some enum with a unique value for each font you are planning to use.  `BitmapFontKey` work
/// similarly, but for bitmap fonts.
///
/// `TexKey` is some type used to identify truetype_fonts. Depending on how many unique textures you plan to
/// have it might be more reasonable to use something like a string type here. Internally, a hash
/// map is used to map from `TexKey`s to actual textures.
pub struct DrawGroup<TruetypeFontKey, BitmapFontKey, TexKey> {
    current_layer: usize,
    layers: [Layer<TruetypeFontKey, BitmapFontKey, TexKey>; LAYER_COUNT],

    // This contains all pushed clip regions that have not yet been popped. 
    // This stack is built up while pushing state commands into the draw group.
    working_clip_stack: Vec<Region>,
    // This stack is only used when drawing, and will go through the same series of transformations
    // as `working_clip_stack` while state commands are played back.
    draw_clip_stack: Vec<Region>,

    shader: Shader,
    truetype_fonts: HashMap<TruetypeFontKey, TruetypeFont>,
    bitmap_fonts: HashMap<BitmapFontKey, BitmapFont>,
    textures: HashMap<TexKey, Texture>,
    white_texture: Texture,

    changed: bool,
    buffer: VertexBuffer<Vert>,
}

#[derive(Debug, Clone)]
struct Layer<TruetypeFontKey, BitmapFontKey, TexKey> {
    vertices: Vec<Vert>,
    state_changes: Vec<StateChange<TruetypeFontKey, BitmapFontKey, TexKey>>,
}

#[derive(Debug, Copy, Clone)]
struct StateChange<TruetypeFontKey, BitmapFontKey, TexKey> {
    at_vertex: usize,
    cmd: StateCmd<TruetypeFontKey, BitmapFontKey, TexKey>,
}

/// Different commands which change drawing state. Commands can be added to a draw group with
/// [`DrawGroup::push_state_cmd`].
///
/// The draw group attempts to ignore unecessarily repeated commands. 
///
/// [`DrawGroup::push_state_cmd`]: struct.DrawGroup.html#method.push_state_cmd
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StateCmd<TruetypeFontKey, BitmapFontKey, TexKey> {
    /// Changes to the given texture. This command is invoked whenever primitives are added to the
    /// draw group with any of the convenience functions (e.g. `line(...)`).
    TextureChange(SamplerId<TruetypeFontKey, BitmapFontKey, TexKey>),

    /// Adds a new item to the clip region stack. 
    PushClip(Region),
    /// Pops one item of the clip region stack, removing the previously pushed clip region. If more
    /// `PopClip` commands than `PushClip` commands are added the draw group will panic.
    PopClip,

    /// Clears the current clip region (Or the entire viewport if there is no clip region)
    /// to the given color.
    Clear(Color),
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum SamplerId<TruetypeFontKey, BitmapFontKey, TexKey> {
    Solid, 
    Texture(TexKey),
    TruetypeFont(TruetypeFontKey),
    BitmapFont(BitmapFontKey),
}

impl<TruetypeFontKey, BitmapFontKey, TexKey> DrawGroup<TruetypeFontKey, BitmapFontKey, TexKey>
  where TruetypeFontKey: Eq + Hash + Copy,
        BitmapFontKey: Eq + Hash + Copy,
        TexKey: Eq + Hash + Copy,
{
    pub fn new() -> Self {
        let shader = build_shader();

        let mut white_texture = Texture::new();
        white_texture.load_data(&[0xff, 0xff, 0xff], 1, 1, TextureFormat::RGB_8);

        // Rust hates me, yada yada. It is not possible to use the [Layer { ... }; 2] syntax though
        let layers = unsafe {
            let layer: Layer<TruetypeFontKey, BitmapFontKey, TexKey> = Layer {
                vertices: Vec::with_capacity(2048),
                state_changes: Vec::with_capacity(256),
            };

            use std::mem;
            use std::ptr;

            let mut layers: [Layer<TruetypeFontKey, BitmapFontKey, TexKey>; LAYER_COUNT] = mem::uninitialized();
            for i in 1..LAYER_COUNT {
                ptr::write((&mut layers[i..]).as_mut_ptr(), layer.clone());
            }
            ptr::write((&mut layers).as_mut_ptr(), layer);

            layers
        }; 

        DrawGroup {
            current_layer: 0,
            layers,

            working_clip_stack: Vec::with_capacity(10), 
            draw_clip_stack:    Vec::with_capacity(10),

            shader,
            white_texture, 
            truetype_fonts: HashMap::new(),
            bitmap_fonts: HashMap::new(),
            textures: HashMap::new(),

            changed: false,
            buffer: VertexBuffer::with_capacity(PrimitiveMode::Triangles, BufferUsage::DynamicDraw, 2048),
        }
    }

    /// Loads a `.ttf` font from the given path and associates it with the given key.
    pub fn load_truetype_font<P: AsRef<Path>>(&mut self, key: TruetypeFontKey, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let font = TruetypeFont::from_file(path)?;

        self.truetype_fonts.insert(key, font);

        Ok(())
    }

    /// Loads a image file from the given path and associates it with the given key.
    pub fn load_texture<P: AsRef<Path>>(&mut self, key: TexKey, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let texture = Texture::from_file(path)?;

        self.textures.insert(key, texture);

        Ok(())
    }

    /// Associates the given font with the given key.
    pub fn include_truetype_font(&mut self, key: TruetypeFontKey, font: TruetypeFont) { 
        self.truetype_fonts.insert(key, font);
    }

    /// Associates the given font with the given key.
    pub fn include_bitmap_font(&mut self, key: BitmapFontKey, font: BitmapFont) { 
        self.bitmap_fonts.insert(key, font);
    }

    /// Associates the given texture with the given key.
    pub fn include_texture(&mut self, key: TexKey, texture: Texture) { 
        self.textures.insert(key, texture);
    }

    /// Removes all vertices and state commands in this group.
    pub fn reset(&mut self) {
        for layer in 0..LAYER_COUNT {
            self.layers[layer].vertices.clear();
            self.layers[layer].state_changes.clear();
        }

        self.changed = true;
        self.working_clip_stack.clear();
    }

    /// Draws all data in this group. This binds a custom shader! `win_size` is just used to reset
    /// the scissor region after rendering.
    pub fn draw(&mut self, transform: Mat4<f32>, win_size: Vec2<f32>) {
        self.draw_clip_stack.clear();

        let total_vert_count: usize = self.layers
            .iter()
            .map(|layer| layer.vertices.len())
            .sum();

        let mut layer_offsets_in_buffer = [0; LAYER_COUNT];

        let mut offset = 0;
        for layer in 0..LAYER_COUNT {
            layer_offsets_in_buffer[layer] = offset;
            offset += self.layers[layer].vertices.len();
        }

        if self.changed {
            self.changed = false;

            self.buffer.clear();
            self.buffer.ensure_allocated(total_vert_count, false);
            for layer in 0..LAYER_COUNT {
                self.buffer.put(layer_offsets_in_buffer[layer], &self.layers[layer].vertices);
            }
        }

        self.shader.bind(); 
        self.shader.set_uniform("transform", transform);

        for layer in 0..LAYER_COUNT {
            graphics::set_scissor(None, win_size);
            self.white_texture.bind(0);
            self.shader.set_uniform("layer", layer as f32 / LAYER_COUNT as f32);

            let mut draw_cursor = 0;
            let ref mut buffer = self.buffer;

            // Draws all data between region start and the given position
            let mut flush = |to: usize| {
                if draw_cursor == to { return; }

                let offset = layer_offsets_in_buffer[layer];

                let start = draw_cursor + offset;
                let end = to + offset;
                buffer.draw_range(start..end);

                draw_cursor = to;
            };

            let mut current_tex = SamplerId::Solid;

            // Process state changes. `flush` whenever we actually change state
            for &StateChange { at_vertex, cmd } in self.layers[layer].state_changes.iter() {
                match cmd {
                    StateCmd::TextureChange(new_tex) => {
                        if new_tex != current_tex {
                            flush(at_vertex);

                            current_tex = new_tex;
                            match current_tex {
                                SamplerId::Solid             => self.white_texture.bind(0),
                                SamplerId::TruetypeFont(key) => self.truetype_fonts[&key].texture().bind(0),
                                SamplerId::BitmapFont(key)   => self.bitmap_fonts[&key].texture.bind(0),
                                SamplerId::Texture(key)      => self.textures[&key].bind(0),
                            }
                        }
                    },

                    StateCmd::Clear(color) => {
                        flush(at_vertex);

                        // Keep in mind that clearing is affected by scissoring
                        graphics::clear(Some(color), true, false);
                    },

                    StateCmd::PushClip(region) => {
                        flush(at_vertex);

                        self.draw_clip_stack.push(region);
                        graphics::set_scissor(Some(region), win_size);
                    },

                    StateCmd::PopClip => {
                        flush(at_vertex);

                        // `pop` returns an option, and thus never panics. We check for unbalanced
                        // push/pops when adding state commands, so at this point we can assume that
                        // they are actually balanced. 
                        self.draw_clip_stack.pop();

                        if let Some(&region) = self.draw_clip_stack.last() {
                            graphics::set_scissor(Some(region), win_size);
                        } else {
                            graphics::set_scissor(None, win_size);
                        }
                    },
                }
            }

            flush(self.layers[layer].vertices.len()); 
        }

        Texture::unbind(0);
        graphics::set_scissor(None, win_size);
    }

    pub fn push_state_cmd(&mut self, cmd: StateCmd<TruetypeFontKey, BitmapFontKey, TexKey>) {
        let ref mut layer = self.layers[self.current_layer];

        // Slight optimization. This is not necessary, as the `draw` function also checks for
        // duplicate values in a more sophisticated way. This just keeps the size of `state_changes`
        // a bit smaller.
        if let Some(&StateChange { cmd: last_cmd, .. }) = layer.state_changes.last() {
            if last_cmd == cmd {
                return;
            }
        }

        match cmd {
            StateCmd::PushClip(region) => {
                self.working_clip_stack.push(region);
            }, 
            StateCmd::PopClip => {
                if self.working_clip_stack.is_empty() {
                    panic!("Unbalanced `StateCmd::PushClip` and `StateCmd::PopClip`");
                }

                self.working_clip_stack.pop();
            },

            _ => {},
        }

        self.changed = true;

        layer.state_changes.push(StateChange {
            at_vertex: layer.vertices.len(),
            cmd: cmd,
        });
    }

    pub fn set_layer(&mut self, layer: usize) {
        assert!(
            layer < LAYER_COUNT,
            "Can not use layers greater than or equal to LAYER_COUNT ({} >= {})",
            layer, LAYER_COUNT
        );

        self.current_layer = layer;
    }

    /// Retrieves a reference to the font, or panics if no font has been registered for the given key.
    pub fn truetype_font(&self, key: TruetypeFontKey) -> &TruetypeFont {
        &self.truetype_fonts[&key]
    }

    /// Retrieves a reference to the font, or panics if no font has been registered for the given key.
    pub fn bitmap_font(&self, key: BitmapFontKey) -> &BitmapFont {
        &self.bitmap_fonts[&key]
    }
    
    /// Retrieves a reference to the texture, or panics if no texture has been registered for the 
    /// given key.
    pub fn texture(&self, key: TexKey) -> &Texture {
        &self.textures[&key]
    }

    /// Retrieves the current clipping rectangle. The returned region is the region to which
    /// vertices will be constrained during drawing. If the clipping stack is empty, this returns 
    /// `None`. The clipping region is changed by pushing [`StateCmd::PushClip`][0] and 
    /// [`StateCmd::PopClip`][0].
    ///
    /// [0]: enum.StateCmd.html
    pub fn current_clip_region(&self) -> Option<Region> {
        match self.working_clip_stack.last() {
            Some(region) => Some(*region),
            None         => None,
        }
    }

    fn add_vertices(&mut self, new: &[Vert]) {
        self.layers[self.current_layer].vertices.extend_from_slice(new);
    }

    /// Draws a thick line.
    pub fn line(&mut self, a: Vec2<f32>, b: Vec2<f32>, width: f32, color: Color) { 
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let normal = (b - a).normalize().left() * (width / 2.0);
        let uv = Vec2::ZERO;
        self.add_vertices(&[
            Vert { pos: a - normal, uv, color },
            Vert { pos: b - normal, uv, color },
            Vert { pos: b + normal, uv, color },
            Vert { pos: a - normal, uv, color },
            Vert { pos: b + normal, uv, color },
            Vert { pos: a + normal, uv, color },
        ]);
    }

    /// Draws a thick line which starts with one color and transitions to another color.
    pub fn multicolor_line(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>,
        width: f32, 
        color_a: Color, color_b: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let normal = (b - a).normalize().left() * (width / 2.0);
        let uv = Vec2::ZERO;
        self.add_vertices(&[
            Vert { pos: a - normal, uv, color: color_a },
            Vert { pos: b - normal, uv, color: color_b },
            Vert { pos: b + normal, uv, color: color_b },
            Vert { pos: a - normal, uv, color: color_a },
            Vert { pos: b + normal, uv, color: color_b },
            Vert { pos: a + normal, uv, color: color_a },
        ]);
    }

    /// Draws a thick line with rounded caps.
    pub fn round_capped_line(&mut self, a: Vec2<f32>, b: Vec2<f32>, width: f32, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid)); 
        let uv = Vec2::ZERO;

        let size = width/2.0;

        let len = (b - a).len();
        let tangent = (b - a) / len; 
        let normal = tangent.left();

        let a = a + tangent*size;
        let b = b - tangent*size;

        // Draw main line
        self.add_vertices(&[
            Vert { pos: a - normal*size, uv, color },
            Vert { pos: b - normal*size, uv, color },
            Vert { pos: b + normal*size, uv, color },
            Vert { pos: a - normal*size, uv, color },
            Vert { pos: b + normal*size, uv, color },
            Vert { pos: a + normal*size, uv, color },
        ]);

        // Draw caps
        for i in 0..(SIN_COS.len() - 1) {
            let c = (
                Vec2::complex_mul(SIN_COS[i], -normal),
                Vec2::complex_mul(SIN_COS[i + 1], -normal)
            );

            let d = (
                Vec2::complex_mul(SIN_COS[i], tangent),
                Vec2::complex_mul(SIN_COS[i + 1], tangent)
            );

            self.add_vertices(&[
                Vert { pos: a, uv, color },
                Vert { pos: a + Vec2::new(-c.0.x, -c.0.y)*size, uv, color },
                Vert { pos: a + Vec2::new(-c.1.x, -c.1.y)*size, uv, color },
                Vert { pos: a, uv, color },
                Vert { pos: a + Vec2::new(-d.0.x, -d.0.y)*size, uv, color },
                Vert { pos: a + Vec2::new(-d.1.x, -d.1.y)*size, uv, color },

                Vert { pos: b, uv, color },
                Vert { pos: b + Vec2::new(c.0.x, c.0.y)*size, uv, color },
                Vert { pos: b + Vec2::new(c.1.x, c.1.y)*size, uv, color },
                Vert { pos: b, uv, color },
                Vert { pos: b + Vec2::new(d.0.x, d.0.y)*size, uv, color },
                Vert { pos: b + Vec2::new(d.1.x, d.1.y)*size, uv, color },
            ]);
        }
    }

    /// Generate the vertices for a stippled line
    pub fn stippled_line(
        &mut self,
        mut a: Vec2<f32>, mut b: Vec2<f32>, 
        width: f32, stipple_length: f32, stipple_spacing: f32, 
        color: Color
    ) { 
        // If we try to draw a very long stippled line this will take up a lot of memory, as each
        // small segment is a separate line. I often accidentally draw a very long line, where the
        // vast majority of it lies offscreen. This is fixed by clipping the line so we only render
        // the minimum required segments to be visible on screen.
        // This might change the apperance of the line slightly because it will shift its segments...
        if let Some(region) = self.working_clip_stack.last() {
            let hit = |pos: Vec2<f32>, dir: Vec2<f32>| -> Option<f32> {
                if region.contains(pos) {
                    return Some(0.0); 
                }

                if pos.x <= region.min.x && dir.x > 0.0 {
                    let t = (region.min.x - pos.x) / dir.x;
                    let y = pos.y + dir.y*t;
                    if t >= 0.0 && t <= 1.0 && y >= region.min.y && y <= region.max.y {
                        return Some(t);
                    }
                }
                if pos.x >= region.max.x && dir.x < 0.0 {
                    let t = (region.max.x - pos.x) / dir.x;
                    let y = pos.y + dir.y*t;
                    if t >= 0.0 && t <= 1.0 && y >= region.min.y && y <= region.max.y {
                        return Some(t);
                    }
                }

                if pos.y <= region.min.y && dir.y > 0.0 {
                    let t = (region.min.y - pos.y) / dir.y;
                    let x = pos.y + dir.y*t;
                    if t >= 0.0 && t <= 1.0 && x >= region.min.x && x <= region.max.x {
                        return Some(t);
                    }
                }
                if pos.y >= region.max.y && dir.y < 0.0 {
                    let t = (region.max.y - pos.y) / dir.y;
                    let x = pos.x + dir.x*t;
                    if t >= 0.0 && t <= 1.0 && x >= region.min.x && x <= region.max.x {
                        return Some(t);
                    }
                }

                return None;
            };

            if let Some(t) = hit(a, b - a) {
                a = Vec2::lerp(a, b, t);
            } else {
                return;
            }

            if let Some(t) = hit(b, a - b) {
                b = Vec2::lerp(b, a, t);
            } else {
                return;
            }
        }

        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid)); 

        let len = (b - a).len(); // The length of the line
        let dir = (b - a) / len; // Unit vector from a to b

        // Just draw a single, slightly extended, segment
        if stipple_length + stipple_spacing > len {
            self.line(a, b, width, color);
        // Create a bunch of line segments, starting at the middle
        } else {
            let mut start = 0.0;
            while start < len/2.0 {
                let end = if start == 0.0 {
                    stipple_length/2.0
                } else {
                    (start + stipple_length).min(len/2.0)
                };

                self.line(a + dir*(len/2.0 + start), a + dir*(len/2.0 + end), width, color);
                self.line(a + dir*(len/2.0 - start), a + dir*(len/2.0 - end), width, color);

                start = end + stipple_spacing;
            }
        } 
    }

    /// Generate the vertices for a stippled line which starts with one color and transitions to
    /// another color.
    pub fn multicolor_stippled_line(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>, 
        width: f32, stipple_length: f32, stipple_spacing: f32, 
        color_a: Color, color_b: Color,
    ) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let len = (b - a).len(); // The length of the line
        let dir = (b - a) / len; // Unit vector from a to b

        // Just draw a single, slightly extended, segment
        if stipple_length + stipple_spacing > len {
            self.multicolor_line(a, b, width, color_a, color_b);

        // Create a bunch of line segments, starting at the middle
        } else {
            let mut start = 0.0;
            while start < len/2.0 {
                let end = if start == 0.0 {
                    stipple_length/2.0
                } else {
                    (start + stipple_length).min(len/2.0)
                };

                let t0 = start / len;
                let t1 = end / len;

                self.multicolor_line(
                    a + dir*(len/2.0 + start), 
                    a + dir*(len/2.0 + end), 
                    width, 
                    Color::lerp(color_a, color_b, 0.5 + t0),
                    Color::lerp(color_a, color_b, 0.5 + t1),
                );

                self.multicolor_line(
                    a + dir*(len/2.0 - start),
                    a + dir*(len/2.0 - end),
                    width, 
                    Color::lerp(color_a, color_b, 0.5 - t0),
                    Color::lerp(color_a, color_b, 0.5 - t1),
                );

                start = end + stipple_spacing;
            }
        } 
    }

    /// Generates the vertices for a square with the given side length centered at the given point.
    pub fn point(&mut self, point: Vec2<f32>, size: f32, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let size = size / 2.0;
        let uv = Vec2::ZERO;
        self.add_vertices(&[
            Vert { pos: point + Vec2::new(-size, -size), uv, color },
            Vert { pos: point + Vec2::new( size, -size), uv, color },
            Vert { pos: point + Vec2::new( size,  size), uv, color },
            Vert { pos: point + Vec2::new(-size, -size), uv, color },
            Vert { pos: point + Vec2::new( size,  size), uv, color },
            Vert { pos: point + Vec2::new(-size,  size), uv, color },
        ]);
    }

    /// Generates the vertices for a circle with the given radius centered at the given position
    pub fn circle(&mut self, pos: Vec2<f32>, radius: f32, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid)); 
        let uv = Vec2::ZERO;

        for i in 0..(SIN_COS.len() - 1) {
            let a = SIN_COS[i];
            let b = SIN_COS[i + 1];

            self.add_vertices(&[
                Vert { pos: pos, uv, color },
                Vert { pos: pos + Vec2::new(a.x, a.y)*radius, uv, color },
                Vert { pos: pos + Vec2::new(b.x, b.y)*radius, uv, color },

                Vert { pos: pos, uv, color },
                Vert { pos: pos + Vec2::new(-a.x, a.y)*radius, uv, color },
                Vert { pos: pos + Vec2::new(-b.x, b.y)*radius, uv, color },

                Vert { pos: pos, uv, color },
                Vert { pos: pos + Vec2::new(a.x, -a.y)*radius, uv, color },
                Vert { pos: pos + Vec2::new(b.x, -b.y)*radius, uv, color },

                Vert { pos: pos, uv, color },
                Vert { pos: pos + Vec2::new(-a.x, -a.y)*radius, uv, color },
                Vert { pos: pos + Vec2::new(-b.x, -b.y)*radius, uv, color },
            ]);
        }
    }

    /// Generates vertices for a line with a arrowhead at `b`.
    pub fn arrow(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>,
        width: f32,
        arrow_size: f32,
        color: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let width = width / 2.0;
        let arrow_size = arrow_size / 2.0;
        let tangent = (b - a).normalize();
        let normal = tangent.left();
        let uv = Vec2::ZERO;

        // Line
        self.line(a, b - tangent*arrow_size, width, color);
        // Arrow head
        self.add_vertices(&[
            Vert { pos: b - tangent*arrow_size - normal*(0.3 * arrow_size), uv, color },
            Vert { pos: b - tangent*arrow_size + normal*(0.3 * arrow_size), uv, color },
            Vert { pos: b, uv, color },
        ]);
    }

    /// Generates vertices for a line with a arrowhead at `b`.
    pub fn stippled_arrow(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>,
        width: f32, stipple_length: f32, stipple_spacing: f32, 
        arrow_size: f32,
        color: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let width = width / 2.0;
        let arrow_size = arrow_size / 2.0;
        let tangent = (b - a).normalize();
        let normal = tangent.left();
        let uv = Vec2::ZERO;

        // Line
        self.stippled_line(a, b - tangent*arrow_size, width, stipple_length, stipple_spacing, color);
        // Arrow head
        self.add_vertices(&[
            Vert { pos: b - tangent*arrow_size - normal*(0.3 * arrow_size), uv, color },
            Vert { pos: b - tangent*arrow_size + normal*(0.3 * arrow_size), uv, color },
            Vert { pos: b, uv, color },
        ]);
    }

    /// Draws a single solid triangle.
    pub fn triangle(&mut self, points: [Vec2<f32>; 3], color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));
        let uv = Vec2::ZERO;

        self.add_vertices(&[
            Vert { pos: points[0], uv, color },
            Vert { pos: points[1], uv, color },
            Vert { pos: points[2], uv, color },
        ]);
    } 

    /// Draws a line loop with neatly connected line corners. This connects the first and last
    /// point in the loop.
    pub fn closed_line_loop(&mut self, points: &[Vec2<f32>], width: f32, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        for i in 0..points.len() {
            let a = points[i]; 
            let b = points[(i+1) % points.len()]; 
            let c = points[(i+2) % points.len()]; 
            let d = points[(i+3) % points.len()]; 

            self.connected_line_segment(a, b, c, d, width, color);
        }
    }
    
    /// Draws a line loop with neatly connected line corners. The first and last points of the loop
    /// are not connected. This is not really a loop.
    pub fn open_line_loop(&mut self, points: &[Vec2<f32>], width: f32, color: Color) {
        if points.len() < 2 {
            return;
        } else if points.len() == 2 {
            self.line(points[0], points[1], width, color);
            return;
        }

        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let b = points[0]; 
        let c = points[1]; 
        let d = points[2]; 
        let a = b*2.0 - c;
        self.connected_line_segment(a, b, c, d, width, color);

        for i in 1..(points.len() - 1) {
            let a = points[(i-1) % points.len()]; 
            let b = points[(i) % points.len()]; 
            let c = points[(i+1) % points.len()]; 
            let d = points[(i+2) % points.len()]; 

            self.connected_line_segment(a, b, c, d, width, color);
        }

        let a = points[points.len() - 3]; 
        let b = points[points.len() - 2]; 
        let c = points[points.len() - 1]; 
        let d = c*2.0 - b;
        self.connected_line_segment(a, b, c, d, width, color);
    }

    /// Draws a line between `b` and `c` which are part of the line semgnet `a b c d`.
    pub fn connected_line_segment(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>,
        c: Vec2<f32>, d: Vec2<f32>,
        width: f32,
        color: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));

        let start_normal = (b - a).left().normalize();
        let center_normal = (c - b).left().normalize();
        let end_normal = (d - c).left().normalize();

        let b_normal = (start_normal + center_normal).normalize();
        let dot = Vec2::dot(b_normal, center_normal);
        let b_normal = b_normal/dot * width/2.0;

        let c_normal = (end_normal + center_normal).normalize();
        let dot = Vec2::dot(c_normal, center_normal);
        let c_normal = c_normal/dot * width/2.0;

        let uv = Vec2::ZERO;

        self.add_vertices(&[
            Vert { pos: b - b_normal, uv, color },
            Vert { pos: c - c_normal, uv, color },
            Vert { pos: c + c_normal, uv, color },
            Vert { pos: b - b_normal, uv, color },
            Vert { pos: c + c_normal, uv, color },
            Vert { pos: b + b_normal, uv, color },
        ]);
    }

    /// Draws borders for an axis align bounding box.
    pub fn line_aabb(&mut self, min: Vec2<f32>, max: Vec2<f32>, width: f32, color: Color) {
        let points = [
            Vec2::new(min.x, min.y),
            Vec2::new(max.x, min.y),
            Vec2::new(max.x, max.y),
            Vec2::new(min.x, max.y),
        ];
        self.closed_line_loop( 
            &points,
            width, color
        ); 
    }

    /// Draws a solid axis-aligned bounding box.
    pub fn aabb(&mut self, min: Vec2<f32>, max: Vec2<f32>, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));
        let uv = Vec2::ZERO;

        self.add_vertices(&[
            Vert { pos: Vec2::new(min.x, min.y), uv, color },
            Vert { pos: Vec2::new(max.x, min.y), uv, color },
            Vert { pos: Vec2::new(max.x, max.y), uv, color },

            Vert { pos: Vec2::new(min.x, min.y), uv, color },
            Vert { pos: Vec2::new(max.x, max.y), uv, color },
            Vert { pos: Vec2::new(min.x, max.y), uv, color },
        ]);
    }

    /// Draws a solid axis-aligned bounding box with rounded corners.
    pub fn rounded_aabb(&mut self, min: Vec2<f32>, max: Vec2<f32>, corner_radius: f32, color: Color) {
        if corner_radius == 0.0 {
            self.aabb(min, max, color);
            return;
        }

        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Solid));
        let uv = Vec2::ZERO;

        self.add_vertices(&[
            // Draw inner + top/bottom border
            Vert { pos: Vec2::new(min.x + corner_radius, min.y), uv, color },
            Vert { pos: Vec2::new(max.x - corner_radius, min.y), uv, color },
            Vert { pos: Vec2::new(max.x - corner_radius, max.y), uv, color },

            Vert { pos: Vec2::new(min.x + corner_radius, min.y), uv, color },
            Vert { pos: Vec2::new(max.x - corner_radius, max.y), uv, color },
            Vert { pos: Vec2::new(min.x + corner_radius, max.y), uv, color },

            // Left border
            Vert { pos: Vec2::new(min.x, min.y + corner_radius), uv, color },
            Vert { pos: Vec2::new(min.x + corner_radius, min.y + corner_radius), uv, color },
            Vert { pos: Vec2::new(min.x + corner_radius, max.y - corner_radius), uv, color },

            Vert { pos: Vec2::new(min.x, min.y + corner_radius), uv, color },
            Vert { pos: Vec2::new(min.x + corner_radius, max.y - corner_radius), uv, color },
            Vert { pos: Vec2::new(min.x, max.y - corner_radius), uv, color },

            // Right border
            Vert { pos: Vec2::new(max.x - corner_radius, min.y + corner_radius), uv, color },
            Vert { pos: Vec2::new(max.x, min.y + corner_radius), uv, color },
            Vert { pos: Vec2::new(max.x, max.y - corner_radius), uv, color },

            Vert { pos: Vec2::new(max.x - corner_radius, min.y + corner_radius), uv, color },
            Vert { pos: Vec2::new(max.x, max.y - corner_radius), uv, color },
            Vert { pos: Vec2::new(max.x - corner_radius, max.y - corner_radius), uv, color },
        ]);

        // Draw corners
        for i in 0..(SIN_COS.len() - 1) {
            let a = SIN_COS[i];
            let b = SIN_COS[i + 1];

            self.add_vertices(&[
                // Top left corner
                Vert { pos: Vec2::new(min.x + corner_radius, min.y + corner_radius), color, uv },
                Vert { pos: Vec2::new(min.x + (1.0 - a.x)*corner_radius, min.y + (1.0 - a.y)*corner_radius), color, uv },
                Vert { pos: Vec2::new(min.x + (1.0 - b.x)*corner_radius, min.y + (1.0 - b.y)*corner_radius), color, uv },
                // Top right corner
                Vert { pos: Vec2::new(max.x - corner_radius, min.y + corner_radius), color, uv },
                Vert { pos: Vec2::new(max.x + (a.x - 1.0)*corner_radius, min.y + (1.0 - a.y)*corner_radius), color, uv },
                Vert { pos: Vec2::new(max.x + (b.x - 1.0)*corner_radius, min.y + (1.0 - b.y)*corner_radius), color, uv },
                // Bottom right corner
                Vert { pos: Vec2::new(max.x - corner_radius, max.y - corner_radius), color, uv },
                Vert { pos: Vec2::new(max.x + (a.x - 1.0)*corner_radius, max.y + (a.y - 1.0)*corner_radius), color, uv },
                Vert { pos: Vec2::new(max.x + (b.x - 1.0)*corner_radius, max.y + (b.y - 1.0)*corner_radius), color, uv },
                // Bottom left corner
                Vert { pos: Vec2::new(min.x + corner_radius, max.y - corner_radius), color, uv },
                Vert { pos: Vec2::new(min.x + (1.0 - a.x)*corner_radius, max.y + (a.y - 1.0)*corner_radius), color, uv },
                Vert { pos: Vec2::new(min.x + (1.0 - b.x)*corner_radius, max.y + (b.y - 1.0)*corner_radius), color, uv },
            ]);
        }
    }

    /// Draws a textured axis-aligned bounding box.
    pub fn textured_aabb(&mut self, texture: TexKey, min: Vec2<f32>, max: Vec2<f32>) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::Texture(texture)));
        let color = Color::rgb(1.0, 1.0, 1.0);

        self.add_vertices(&[
            Vert { pos: Vec2::new(min.x, min.y), color, uv: Vec2::new(0.0, 0.0) },
            Vert { pos: Vec2::new(max.x, min.y), color, uv: Vec2::new(1.0, 0.0) },
            Vert { pos: Vec2::new(max.x, max.y), color, uv: Vec2::new(1.0, 1.0) },

            Vert { pos: Vec2::new(min.x, min.y), color, uv: Vec2::new(0.0, 0.0) },
            Vert { pos: Vec2::new(max.x, max.y), color, uv: Vec2::new(1.0, 1.0) },
            Vert { pos: Vec2::new(min.x, max.y), color, uv: Vec2::new(0.0, 1.0) },
        ]);
    }

    pub fn truetype_text(
        &mut self,
        text: &str,
        font: TruetypeFontKey,
        size: f32,
        pos: Vec2<f32>,
        wrap_width: Option<f32>,
        color: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::TruetypeFont(font)));

        let ref mut vertices = self.layers[self.current_layer].vertices;
        let callback = |pos, uv| vertices.push(Vert { pos, uv, color });

        self.truetype_fonts.get_mut(&font).unwrap().cache(
            text,
            size, 1.0, 
            pos.round(), // By rounding we avoid a lot of nasty subpixel issues.
            wrap_width,
            callback,
        ); 
    }

    pub fn bitmap_text(&mut self, text: &str, font: BitmapFontKey, pos: Vec2<f32>, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(SamplerId::BitmapFont(font)));

        let ref mut vertices = self.layers[self.current_layer].vertices;
        let callback = |pos, uv| vertices.push(Vert { pos, uv, color });

        self.bitmap_fonts.get_mut(&font).unwrap().cache(
            text,
            pos.round(), // By rounding we avoid a lot of nasty subpixel issues.
            callback,
        ); 
    }
}

/// For angles from 0 to π/2
const SIN_COS: [Vec2<f32>; 11] = [
    Vec2 { x: 1.00000000, y: 0.00000000 },
    Vec2 { x: 0.98768836, y: 0.15643448 },
    Vec2 { x: 0.95105654, y: 0.30901700 },
    Vec2 { x: 0.89100653, y: 0.45399055 },
    Vec2 { x: 0.80901700, y: 0.58778524 },
    Vec2 { x: 0.70710677, y: 0.70710677 },
    Vec2 { x: 0.58778518, y: 0.80901706 },
    Vec2 { x: 0.45399052, y: 0.89100653 },
    Vec2 { x: 0.30901697, y: 0.95105654 },
    Vec2 { x: 0.15643449, y: 0.98768836 },
    Vec2 { x: 0.00000000, y: 1.00000000 },
];

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Vert {
    pub pos: Vec2<f32>,
    pub uv: Vec2<f32>,
    pub color: Color,
}

// We cannot use the custom derive from within this crate :/
impl Vertex for Vert {
    fn setup_attrib_pointers(divisor: usize) {
        use std::mem;

        use gl;

        let stride = mem::size_of::<Vert>();
        let mut offset = 0;

        AttribBinding {
            index: 0,
            primitives: 2,
            primitive_type: gl::FLOAT,
            normalized: false,
            integer: false,
            stride, offset, divisor,
        }.enable();
        offset += mem::size_of::<Vec2<f32>>();

        AttribBinding {
            index: 1,
            primitives: 2,
            primitive_type: gl::FLOAT,
            normalized: false,
            integer: false,
            stride, offset, divisor,
        }.enable();
        offset += mem::size_of::<Vec2<f32>>();

        AttribBinding {
            index: 2,
            primitives: 4,
            primitive_type: gl::FLOAT,
            normalized: false,
            integer: false,
            stride, offset, divisor,
        }.enable();
    }

    // Not used, we manualy declare inputs in the shader
    fn gen_shader_input_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_outputs(_name_prefix: &str) -> Vec<String> { Vec::new() }
    fn set_as_vertex_attrib(&self) {}
}

const VERT_SRC: &'static str = "
    #version 330 core

    layout(location = 0) in vec2 in_pos;
    layout(location = 1) in vec2 in_uv;
    layout(location = 2) in vec4 in_color;

    out vec4 v_color;
    out vec2 v_uv;

    uniform mat4 transform;
    uniform float layer = 0.0;

    void main() {
        gl_Position = transform * vec4(in_pos, layer, 1.0);
        v_color = in_color;
        v_uv = in_uv;
    }
";

const FRAG_SRC: &'static str = "
    #version 330 core

    in vec2 v_uv;
    in vec4 v_color;

    out vec4 color;

    uniform sampler2D texture_sampler;

    void main() {
        color = v_color * texture(texture_sampler, v_uv);
    }
";

fn build_shader() -> Shader {
    let proto = ShaderPrototype::new_prototype(VERT_SRC, "", FRAG_SRC);
    match proto.build() {
        Ok(shader) => {
            shader
        },
        Err(err) => {
            // We should only ever panic if the code of the shader declared above is invalid, in
            // which should be caught during testing.
            // Print the error properly before panicing.
            println!("{}", err); 
            panic!();
        }
    }
}

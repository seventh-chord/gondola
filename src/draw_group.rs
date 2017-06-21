
//! Utilities for graphics

use std::f32;
use std::io;
use std::path::Path;
use std::hash::Hash;
use std::collections::HashMap;

use cable_math::{Vec2, Mat3, Mat4};

use Color;
use graphics; 
use Region;
use shader::{ShaderPrototype, Shader};
use texture::{Texture, TextureFormat};
use buffer::{Vertex, PrimitiveMode, BufferUsage, VertexBuffer};
use font::{Font, AsFontVert};

/// Batches drawcalls of textured primitives.
///
/// `F` is some type used to identify fonts. Typically you would want to use some enum with a
/// unique value for each font you are planning to use.
pub struct DrawGroup<F> {
    vertices: Vec<Vert>,
    state_changes: Vec<StateChange<F>>,

    // This contains all pushed clip regions that have not yet been popped. 
    // This stack is built up while pushing state commands into the draw group.
    working_clip_stack: Vec<Region>,
    // This stack is only used when drawing, and will go through the same series of transformations
    // as `working_clip_stack` while state commands are played back.
    draw_clip_stack: Vec<Region>,

    /// If set to some transformation matrix, that transform will be applied to all vertices when
    /// they are added to this draw group. 
    pub current_transform: Option<Mat3<f32>>,

    shader: Shader,
    fonts: HashMap<F, Font>,
    white_texture: Texture,

    changed: bool,
    buffer: VertexBuffer<Vert>,
}

#[derive(Debug, Copy, Clone)]
struct StateChange<F> {
    at_vertex: usize,
    cmd: StateCmd<F>,
}

/// Different commands which change drawing state. Commands can be added to a draw group with
/// [`DrawGroup::push_state_cmd`].
///
/// The draw group attempts to ignore unecessarily repeated commands. 
///
/// [`DrawGroup::push_state_cmd`]: struct.DrawGroup.html#method.push_state_cmd
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StateCmd<F> {
    /// Changes to the given texture. This command is invoked whenever primitives are added to the
    /// draw group with any of the convenience functions (e.g. `line(...)`).
    TextureChange(TextureId<F>),

    /// Adds a new item to the clip region stack. 
    PushClip(Region),
    /// Pops one item of the clip region stack, removing the previously pushed clip region. If more
    /// `PopClip` commands than `PushClip` commands are added the draw group will panic.
    PopClip,

    /// Clears the current clip region (Or the entire viewport if there is no clip region)
    /// to the given color.
    Clear(Color),

    /// Changes the layer (The z coordinate of all vertices). This can be used to place some
    /// sections above others when rendering. `0.0` is the default layer.
    LayerChange(f32),
}

impl<F> DrawGroup<F> 
  where F: Eq + Hash + Copy,
{
    pub fn new() -> Self {
        let shader = build_shader();

        let mut white_texture = Texture::new();
        white_texture.load_data(&[0xff, 0xff, 0xff], 1, 1, TextureFormat::RGB_8);

        DrawGroup {
            vertices: Vec::with_capacity(2048),
            state_changes: Vec::with_capacity(256),

            working_clip_stack: Vec::with_capacity(10), 
            draw_clip_stack:    Vec::with_capacity(10),

            current_transform: None,

            shader,
            white_texture, 
            fonts: HashMap::new(),

            changed: false,
            buffer: VertexBuffer::with_capacity(PrimitiveMode::Triangles, BufferUsage::DynamicDraw, 2048),
        }
    }

    pub fn load_font<P: AsRef<Path>>(&mut self, key: F, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let font = Font::from_file(path)?;

        self.fonts.insert(key, font);

        Ok(())
    }

    /// Removes all vertices and state commands in this group.
    pub fn reset(&mut self) {
        self.vertices.clear();
        self.state_changes.clear(); 
        self.changed = true;
        self.working_clip_stack.clear();
    }

    /// Draws all data in this group. This expects the proper shader (basic.glsl) to be bound.
    pub fn draw(&mut self, transform: Mat4<f32>, win_size: Vec2<f32>) {
        self.draw_clip_stack.clear();

        if self.changed {
            self.changed = false;

            self.buffer.clear();
            self.buffer.put(0, &self.vertices);
        }

        self.shader.bind(); 
        self.shader.set_uniform("transform", transform);
        self.shader.set_uniform("layer", 0.0);
        self.white_texture.bind(0);

        let mut draw_cursor = 0;
        let ref mut buffer = self.buffer;

        // Draws all data between region start and the given position
        let mut flush = |to: usize| {
            if draw_cursor == to { return; }

            buffer.draw_range(draw_cursor..to);
            draw_cursor = to;
        };

        let mut current_tex = TextureId::Solid;
        let mut current_layer = 0.0;

        // Process state changes. `flush` whenever we actually change state
        for &StateChange { at_vertex, cmd } in self.state_changes.iter() {
            match cmd {
                StateCmd::TextureChange(new_tex) => {
                    if new_tex != current_tex {
                        flush(at_vertex);

                        current_tex = new_tex;
                        match current_tex {
                            TextureId::Solid     => self.white_texture.bind(0),
                            TextureId::Font(key) => self.fonts[&key].texture().bind(0),
                        }
                    }
                },

                StateCmd::LayerChange(new_layer) => {
                    if new_layer != current_layer {
                        flush(at_vertex);

                        current_layer = new_layer;
                        self.shader.set_uniform("layer", current_layer);
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

        flush(self.vertices.len()); 

        Texture::unbind(0);
        graphics::set_scissor(None, win_size);
    }

    pub fn push_state_cmd(&mut self, cmd: StateCmd<F>) {
        // Slight optimization. This is not necessary, as the `draw` function also checks for
        // duplicate values in a more sophisticated way. This just keeps the size of `state_changes`
        // a bit smaller.
        if let Some(&StateChange { cmd: last_cmd, .. }) = self.state_changes.last() {
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
        self.state_changes.push(StateChange {
            at_vertex: self.vertices.len(),
            cmd: cmd,
        });
    }

    pub fn font(&self, key: F) -> &Font {
        &self.fonts[&key]
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

    fn add_vertices(&mut self, vertices: &[Vert]) {
        if let Some(transform) = self.current_transform {
            for v in vertices.into_iter() {
                self.vertices.push(Vert {
                    pos: transform.apply(v.pos),
                    .. *v
                });
            } 
        } else {
            self.vertices.extend_from_slice(vertices);
        }
    }

    /// Draws a thick line.
    pub fn line(&mut self, a: Vec2<f32>, b: Vec2<f32>, width: f32, color: Color) { 
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        let normal = (b - a).normalize().left() * (width / 2.0);
        let uv = Vec2::zero();
        self.add_vertices(&[
            Vert { pos: a - normal, uv, color },
            Vert { pos: b - normal, uv, color },
            Vert { pos: b + normal, uv, color },
            Vert { pos: a - normal, uv, color },
            Vert { pos: b + normal, uv, color },
            Vert { pos: a + normal, uv, color },
        ]);
    }

    /// Draws a thick line with different colors at each end.
    pub fn multicolor_line(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>,
        width: f32, 
        color_a: Color, color_b: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        let normal = (b - a).normalize().left() * (width / 2.0);
        let uv = Vec2::zero();
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
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid)); 
        let uv = Vec2::zero();

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
            let ca = Vec2::complex_mul(SIN_COS[i], -normal);
            let cb = Vec2::complex_mul(SIN_COS[i + 1], -normal);

            self.add_vertices(&[
                Vert { pos: a, uv, color },
                Vert { pos: a + Vec2::new(-ca.x, ca.y)*size, uv, color },
                Vert { pos: a + Vec2::new(-cb.x, cb.y)*size, uv, color },
                Vert { pos: a, uv, color },
                Vert { pos: a + Vec2::new(-cb.x, -cb.y)*size, uv, color },
                Vert { pos: a + Vec2::new(-ca.x, -ca.y)*size, uv, color },

                Vert { pos: b, uv, color },
                Vert { pos: b + Vec2::new(cb.x, cb.y)*size, uv, color },
                Vert { pos: b + Vec2::new(ca.x, ca.y)*size, uv, color },
                Vert { pos: b, uv, color },
                Vert { pos: b + Vec2::new(ca.x, -ca.y)*size, uv, color },
                Vert { pos: b + Vec2::new(cb.x, -cb.y)*size, uv, color },
            ]);
        }
    }

    /// Generate the vertices for a stippled line
    pub fn stippled_line(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>, 
        width: f32, stipple_length: f32, stipple_spacing: f32, 
        color: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

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

    /// Generates the vertices for a square with the given side length centered at the given point.
    pub fn point(&mut self, point: Vec2<f32>, size: f32, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        let size = size / 2.0;
        let uv = Vec2::zero();
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
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid)); 
        let uv = Vec2::zero();

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
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        let width = width / 2.0;
        let arrow_size = arrow_size / 2.0;
        let tangent = (b - a).normalize();
        let normal = tangent.left();
        let uv = Vec2::zero();

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
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        let width = width / 2.0;
        let arrow_size = arrow_size / 2.0;
        let tangent = (b - a).normalize();
        let normal = tangent.left();
        let uv = Vec2::zero();

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
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));
        let uv = Vec2::zero();

        self.add_vertices(&[
            Vert { pos: points[0], uv, color },
            Vert { pos: points[1], uv, color },
            Vert { pos: points[2], uv, color },
        ]);
    } 

    /// Draws a line loop with neatly connected line corners. This connects the first and last
    /// point in the loop.
    pub fn closed_line_loop(&mut self, points: &[Vec2<f32>], width: f32, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        for i in 0..points.len() {
            let a = points[i]; 
            let b = points[(i+1) % points.len()]; 
            let c = points[(i+2) % points.len()]; 
            let d = points[(i+3) % points.len()]; 

            self.connected_line_segment(a, b, c, d, width, color);
        }
    }

    /// Draws a line between `b` and `c` which are part of the line semgnet `a b c d`.
    pub fn connected_line_segment(
        &mut self,
        a: Vec2<f32>, b: Vec2<f32>,
        c: Vec2<f32>, d: Vec2<f32>,
        width: f32,
        color: Color
    ) {
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));

        let start_normal = (b - a).left().normalize();
        let center_normal = (c - b).left().normalize();
        let end_normal = (d - c).left().normalize();

        let b_normal = (start_normal + center_normal).normalize();
        let dot = Vec2::dot(b_normal, center_normal);
        let b_normal = b_normal/dot * width/2.0;

        let c_normal = (end_normal + center_normal).normalize();
        let dot = Vec2::dot(c_normal, center_normal);
        let c_normal = c_normal/dot * width/2.0;

        let uv = Vec2::zero();

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
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));
        let uv = Vec2::zero();

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

        self.push_state_cmd(StateCmd::TextureChange(TextureId::Solid));
        let uv = Vec2::zero();

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

    pub fn text(&mut self, text: &str, font: F, size: f32, pos: Vec2<f32>, color: Color) {
        self.push_state_cmd(StateCmd::TextureChange(TextureId::Font(font)));
        let count = self.fonts.get_mut(&font).unwrap().cache(
            &mut self.vertices,
            text,
            size, 1.0, 
            pos.round(), // By rounding we avoid a lot of nasty subpixel issues.
            color
        ); 

        if let Some(transform) = self.current_transform {
            for i in self.vertices.len()-count-1 .. self.vertices.len() {
                self.vertices[i].pos = transform.apply(self.vertices[i].pos);
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum TextureId<F> {
    Solid, 
    Font(F),
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

// This allows us to draw text straight into the draw group
impl AsFontVert for Vert {
    fn gen(pos: Vec2<f32>, uv: Vec2<f32>, color: Color) -> Vert{ Vert { pos, uv, color } }
}

// We cannot use the custom derive from within this crate :/
impl Vertex for Vert {
    fn bytes_per_vertex() -> usize { 
        use std::mem;
        mem::size_of::<Vert>() 
    }

    fn setup_attrib_pointers() {
        use std::mem;

        use gl;
        use gl::types::*; 

        let stride = <Vert as Vertex>::bytes_per_vertex();
        let mut offset = 0;
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, offset as *const GLvoid);
            offset += mem::size_of::<Vec2<f32>>();

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, offset as *const GLvoid);
            offset += mem::size_of::<Vec2<f32>>();

            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 4, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, offset as *const GLvoid);
        }
    }
    // Not used, we manualy declare inputs in the shader
    fn gen_shader_input_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_outputs(_name_prefix: &str) -> Vec<String> { Vec::new() }
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
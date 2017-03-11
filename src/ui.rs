
//! Immediate mode gui. See [`ui::Ui`](struct.Ui.html) for more info.

use std::mem;
use gl;
use gl::types::*;
use std::ops::Range;
use std::collections::HashMap;
use std::fmt::Write;
use cable_math::Vec2;

use color::Color;
use font::{Font, CachedFont};
use input::{InputManager, State};
use matrix_stack::MatrixStack;
use shader::{Shader, ShaderPrototype};
use buffer::{Vertex, VertexBuffer, PrimitiveMode, BufferUsage};

const FONT_SIZE: f32 = 14.0;

/// A struct for using a imediate mode gui. 
pub struct Ui {
    pub style: Style,

    mat_stack: MatrixStack,
    font: CachedFont,
    shader: Shader,
    draw_data: Vec<Vert>,
    draw_vbo: VertexBuffer<Vert>,

    caret: Vec2<f32>,
    caret_start: Vec2<f32>,
    line_size: f32,
    line_dir: LineDir,
    held: Option<Id>,

    internal_fmt_string: String,
    slider_map: HashMap<Id, f32>,

    // Input state
    mouse_pos: Vec2<f32>,
    mouse_state: State,
}

impl Ui {
    /// Creates a new imediate mode gui system with the given font. Note that the font will be
    /// copied internally, so you can pass a reference to a font you are using elsewhere in your
    /// program.
    pub fn new(font: &Font) -> Ui {
        Ui {
            style: Default::default(),

            mat_stack: MatrixStack::new(),
            font: CachedFont::from_font(font.clone()),
            shader: build_shader(),
            draw_data: Vec::with_capacity(500),
            draw_vbo: VertexBuffer::with_capacity(PrimitiveMode::Triangles, BufferUsage::DynamicDraw, 500),

            caret: Vec2::zero(),
            caret_start: Vec2::zero(),
            line_size: 0.0,
            line_dir: LineDir::Vertical,
            held: None,

            internal_fmt_string: String::new(),
            slider_map: HashMap::new(),

            mouse_pos: Vec2::zero(),
            mouse_state: State::Up,
        }
    }

    /// Updates this imgui system. This should be called once per frame, before using any of the
    /// gui creation functions.
    pub fn update(&mut self, input: &InputManager, window_size: Vec2<u32>) {
        self.mat_stack.ortho(0.0, window_size.x as f32, 0.0, window_size.y as f32, -1.0, 1.0);

        self.mouse_pos = input.mouse_pos();
        self.mouse_state = input.mouse_key(0);

        if self.mouse_state.up() && !self.mouse_state.released() {
            self.held = None;
        }

        self.caret = Vec2::zero();
    }

    /// Shows all components added since the last call to `draw`. This function update the matrix
    /// buffers and binds new shaders. No special opengl state is required to be set when calling
    /// this function. Note that this function does not necessarily reset the state it changes.
    pub fn draw(&mut self) {
        self.mat_stack.update_buffer();

        self.draw_vbo.clear();
        self.draw_vbo.put(0, &self.draw_data);
        self.draw_data.clear();

        self.shader.bind();
        self.draw_vbo.draw();
        self.font.draw();
    }

    /// Moves the internal caret to the given position. Consecutive items will be inserted at
    /// the caret.
    pub fn set_caret(&mut self, pos: Vec2<f32>, line_dir: LineDir) {
        self.caret = pos;
        self.caret_start = pos;
        self.line_dir = line_dir;
        self.line_size = 0.0;
    }

    /// Advances the caret to the next line. The direction of a line depends on the line direction
    /// set by [`set_caret`].
    ///
    /// [`set_caret`]: struct.Ui.html#method.set_caret
    pub fn next_line(&mut self) {
        match self.line_dir {
            LineDir::Horizontal => {
                self.caret.y += self.line_size + self.style.line_spacing;
                self.caret.x = self.caret_start.x;
                self.line_size = 0.0;
            },
            LineDir::Vertical => {
                self.caret.x += self.line_size + self.style.line_spacing;
                self.caret.y = self.caret_start.y;
                self.line_size = 0.0;
            },
        }
    }

    /// Shows a new button with the given text at the given location. Returns true if the button
    /// was pressed. Note that this function needs to be called every frame you want to see the
    /// button.
    pub fn button(&mut self, text: &str) -> bool {
        let id = Id::from_str(text, CompType::Button);

        let width = self.font.font().width(text, FONT_SIZE) + self.style.internal_padding.x;
        let height = self.default_height();
        let pos = self.caret;
        self.advance_caret(width, height);

        let hovered = self.mouse_pos.x > pos.x && self.mouse_pos.y > pos.y && 
                      self.mouse_pos.x < pos.x + width && self.mouse_pos.y < pos.y + height;
        if hovered && self.mouse_state.pressed() {
            self.held = Some(id);
        }

        let color = if self.held == Some(id) {
            self.style.hold_color
        } else if hovered {
            self.style.hover_color
        } else {
            self.style.base_color
        };
        self.draw_comp(pos, width, height, color, text, Alignment::Left);

        self.held == Some(id) && hovered && self.mouse_state.released()
    }

    /// Creates a new slider that allows selecting values from the given range 
    pub fn slider(&mut self, text: &str, range: Range<f32>) -> f32 {
        let id = Id::from_str(text, CompType::Slider);
        let mut value = *self.slider_map.entry(id).or_insert((range.start + range.end) / 2.0);

        let width = self.style.default_comp_width;
        let height = self.default_height();
        let pos = self.caret;
        self.advance_caret(width, height);

        let hovered = self.mouse_pos.x > pos.x && self.mouse_pos.y > pos.y && 
                      self.mouse_pos.x < pos.x + width && self.mouse_pos.y < pos.y + height;
        if hovered && self.mouse_state.pressed() {
            self.held = Some(id);
        } 

        let slider_size = {
            let size = height - self.style.internal_padding.y;
            Vec2::new(size, size)
        };
        let slider_pos = {
            let norm_value = (value - range.start) / (range.end - range.start);
            let slide_distance = width - self.style.internal_padding.x - slider_size.x;
            pos + Vec2::new(self.style.internal_padding.x/2.0 + norm_value*slide_distance, self.style.internal_padding.y/2.0)
        };

        self.internal_fmt_string.clear();
        write!(self.internal_fmt_string, "{}: {:.*}", text, 2, value).unwrap();

        if self.held == Some(id) {
            value = (self.mouse_pos.x - pos.x - self.style.internal_padding.x/2.0 - slider_size.x/2.0) /
                    (width - self.style.internal_padding.x - slider_size.x);
            if value > 1.0 { value = 1.0 }
            if value < 0.0 { value = 0.0 }
            value = range.start + value*(range.end - range.start);

            self.slider_map.insert(id, value);
        }

        // Main bar
        let color = if hovered { self.style.hover_color } else { self.style.base_color };
        let text = &self.internal_fmt_string.clone();
        self.draw_comp(pos, width, height, color, text, Alignment::Center);
        // Slidy thing
        let color = if self.held == Some(id) { self.style.top_hold_color } else { self.style.top_color };
        quad(&mut self.draw_data, slider_pos, slider_size, color);

        value
    }

    fn draw_comp(&mut self, pos: Vec2<f32>, width: f32, height: f32, color: Color, text: &str, alignment: Alignment) {
        quad(&mut self.draw_data, pos, Vec2::new(width, height), color);
        match alignment {
            Alignment::Left => {
                let text_start = self.style.internal_padding.y/2.0 - self.font.font().descent(FONT_SIZE);
                self.font.cache(text, FONT_SIZE, pos + Vec2::new(self.style.internal_padding.x/2.0, height - text_start));
            },
            Alignment::Right => {
                let text_pos = {
                    let text_width = self.font.font().width(&self.internal_fmt_string, FONT_SIZE);
                    let text_v_offset = self.style.internal_padding.y/2.0 - self.font.font().descent(FONT_SIZE);
                    pos + Vec2::new(width - self.style.internal_padding.x/2.0 - text_width, height - text_v_offset)
                }; 
                self.font.cache(text, FONT_SIZE, text_pos);
            },
            Alignment::Center => {
                let text_pos = {
                    let text_width = self.font.font().width(&self.internal_fmt_string, FONT_SIZE);
                    let text_v_offset = self.style.internal_padding.y/2.0 - self.font.font().descent(FONT_SIZE);
                    pos + Vec2::new(width/2.0 - text_width/2.0, height - text_v_offset)
                }; 
                self.font.cache(text, FONT_SIZE, text_pos);
            },
        }
    }

    fn advance_caret(&mut self, comp_width: f32, comp_height: f32) {
        match self.line_dir {
            LineDir::Horizontal => {
                self.caret.x += comp_width + self.style.line_spacing;
                self.line_size = f32::max(comp_height, self.line_size);
            },
            LineDir::Vertical => {
                self.caret.y += comp_height + self.style.line_spacing;
                self.line_size = f32::max(comp_width, self.line_size);
            },
        }
    }

    fn default_height(&self) -> f32 {
        self.font.font().line_height(FONT_SIZE) + self.style.internal_padding.y
    }
}

#[derive(Clone, Debug)]
pub struct Style {
    pub base_color: Color,
    pub hover_color: Color,
    pub hold_color: Color,
    pub top_color: Color,
    pub top_hold_color: Color,

    pub internal_padding: Vec2<f32>,
    pub line_spacing: f32,
    pub default_comp_width: f32,
}
impl Default for Style {
    fn default() -> Style {
        Style {
            base_color:      Color::hex("4c4665"),
            hover_color:     Color::hex("575074"),
            hold_color:      Color::hex("413c56"),
            top_color:       Color::hex("403147"),
            top_hold_color:  Color::hex("2a2738"),

            internal_padding: Vec2::new(10.0, 6.0),
            line_spacing: 5.0,
            default_comp_width: 150.0,
        }
    }
}

pub enum LineDir {
    /// Components are layed out below each other
    Vertical,
    /// Components are layed out side by side
    Horizontal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Alignment {
    Left, Center, Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct Id(u64, CompType);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum CompType {
    Button,
    Slider,
}

impl Id {
    fn from_str(text: &str, ty: CompType) -> Id {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hasher, Hash};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let id = hasher.finish();

        Id(id, ty)
    }
}

#[derive(Debug)]
#[repr(C)]
struct Vert {
    pos: Vec2<f32>,
    color: Color,
}

fn quad(buf: &mut Vec<Vert>, pos: Vec2<f32>, size: Vec2<f32>, color: Color){
    let min = pos;
    let max = pos + size;

    buf.push(Vert { pos: Vec2::new(min.x, min.y), color: color });
    buf.push(Vert { pos: Vec2::new(max.x, min.y), color: color });
    buf.push(Vert { pos: Vec2::new(max.x, max.y), color: color });

    buf.push(Vert { pos: Vec2::new(min.x, min.y), color: color });
    buf.push(Vert { pos: Vec2::new(max.x, max.y), color: color });
    buf.push(Vert { pos: Vec2::new(min.x, max.y), color: color });
}

// We cannot use the custom derive from within this crate
impl Vertex for Vert {
    fn bytes_per_vertex() -> usize { mem::size_of::<Vert>() }
    fn setup_attrib_pointers() {
        let stride = <Vert as Vertex>::bytes_per_vertex();
        let mut offset = 0;
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, offset as *const GLvoid);
            offset += mem::size_of::<Vec2<f32>>();

            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 4, gl::FLOAT,
                                    false as GLboolean,
                                    stride as GLsizei, offset as *const GLvoid);
        }
    }
    // Not used, we manualy declare inputs in the shader
    fn gen_shader_input_decl() -> String { String::new() }
}

const VERT_SRC: &'static str = "
    #version 330 core

    layout(location = 0) in vec2 pos;
    layout(location = 1) in vec4 color;

    out vec4 vert_col;

    // Matrix block is inserted automatically

    void main() {
        gl_Position = mvp * vec4(pos, 0.0, 1.0);
        vert_col = color;
    }
";
const FRAG_SRC: &'static str = "
    #version 330 core

    in vec4 vert_col;
    out vec4 color;

    void main() {
        color = vert_col;
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


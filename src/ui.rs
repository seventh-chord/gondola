
//! Immediate mode gui. See [`ui::Ui`](struct.Ui.html) for more info.

use std::mem;
use gl;
use gl::types::*;
use cable_math::Vec2;

use color::Color;
use font::{Font, CachedFont};
use input::{InputManager, Key, State};
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

    held: Option<Id>,

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

            held: None,

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

    /// Shows a new button with the given text at the given location. Returns true if the button
    /// was pressed. Note that this function needs to be called every frame you want to see the
    /// button.
    pub fn button(&mut self, text: &str, pos: Vec2<f32>) -> bool {
        let width = self.font.font().width(text, FONT_SIZE) + self.style.padding.x;
        let height = self.font.font().line_height(FONT_SIZE) + self.style.padding.y;
        let text_start = self.style.padding.y/2.0 - self.font.font().descent(FONT_SIZE);
        
        let hovered = self.mouse_pos.x > pos.x && self.mouse_pos.y > pos.y && 
                      self.mouse_pos.x < pos.x + width && self.mouse_pos.y < pos.y + height;
        let held = hovered && self.mouse_state.down();

        let color = if held {
            self.style.hold_color
        } else if hovered {
            self.style.hover_color
        } else {
            self.style.button_color
        };

        quad(&mut self.draw_data, pos, Vec2::new(width, height), color);
        self.font.cache(text, FONT_SIZE, pos + Vec2::new(self.style.padding.x/2.0, height - text_start));

        let id = Id::from_str(text, CompType::Button);

        if hovered && self.mouse_state.pressed() {
            self.held = Some(id);
        }

        self.held == Some(id) && hovered && self.mouse_state.released()
    }
}

#[derive(Clone, Debug)]
pub struct Style {
    pub button_color: Color,
    pub hover_color: Color,
    pub hold_color: Color,

    pub padding: Vec2<f32>,
}
impl Default for Style {
    fn default() -> Style {
        Style {
            button_color: Color::hex("4c4665"),
            hover_color: Color::hex("575074"),
            hold_color: Color::hex("413c56"),
            padding: Vec2::new(10.0, 6.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Id(u64, CompType);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompType {
    Button,
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


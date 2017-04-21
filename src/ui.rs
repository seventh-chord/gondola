
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
use input::{InputManager, Key, State};
use matrix_stack::MatrixStack;
use shader::{Shader, ShaderPrototype};
use buffer::{Vertex, VertexBuffer, PrimitiveMode, BufferUsage};

const FONT_SIZE: f32 = 14.0;
const CARET_BLINK_RATE: f32 = 0.53;

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
    focused: Option<Id>,

    salt: String,
    internal_fmt_string: String,
    slider_map: HashMap<Id, f32>,

    textbox_map: HashMap<Id, TextboxInfo>,
    caret_blink_time: f32,

    // Input state
    mouse_pos: Vec2<f32>,
    mouse_state: State,
    move_left: bool, move_right: bool,
    typed: String,
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
            focused: None,

            salt: String::new(),
            internal_fmt_string: String::new(),
            slider_map: HashMap::new(),
            textbox_map: HashMap::new(),
            caret_blink_time: 0.0,

            mouse_pos: Vec2::zero(),
            mouse_state: State::Up,
            move_left: false, move_right: false,
            typed: String::new(),
        }
    }

    /// Updates this imgui system. This should be called once per frame, before using any of the
    /// gui creation functions.
    ///
    /// `delta` should be the time since the last call to `update`, in seconds.
    pub fn update(&mut self, delta: f32, input: &InputManager, window_size: Vec2<u32>) {
        self.mat_stack.ortho(0.0, window_size.x as f32, 0.0, window_size.y as f32, -1.0, 1.0);

        self.mouse_pos = input.mouse_pos();
        self.mouse_state = input.mouse_key(0);
        self.typed.clear();
        self.typed.push_str(input.typed());
        self.move_left = input.key(Key::Left).pressed_repeat();
        self.move_right = input.key(Key::Right).pressed_repeat();

        if self.mouse_state.up() && !self.mouse_state.released() {
            self.held = None;
        }

        if let Some(held) = self.held {
            if Some(held) != self.focused {
                self.focused = None;
            }
        }

        self.caret = Vec2::zero();
        self.caret_blink_time += delta;
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

    /// Sets a string which is used to salt all names when producing ids. Multiple components of
    /// the same type can have the same name as long as a different salt is set when adding each
    /// of the components. Note that the same salt should be used for each component every frame.
    pub fn set_salt(&mut self, salt: &str) {
        self.salt = salt.to_owned();
    }

    /// Moves the internal caret to the given position. Consecutive items will be inserted at
    /// the caret.
    pub fn set_caret(&mut self, pos: Vec2<f32>, line_dir: LineDir) {
        self.caret = pos;
        self.caret_start = pos;
        self.line_dir = line_dir;
        self.line_size = 0.0;
    }

    /// Inserts a empty, invisible box. This only serves to advance the carret and create blank
    /// space inside a complex ui.
    pub fn spacer(&mut self, width: f32, height: f32) {
        self.advance_caret(width, height);
    }

    /// Advances the caret to the next line. The direction of a line depends on the line direction
    /// set by [`set_caret`].
    ///
    /// [`set_caret`]: struct.Ui.html#method.set_caret
    pub fn next_line(&mut self) {
        match self.line_dir {
            LineDir::Horizontal => {
                self.caret.y += self.line_size + self.style.margin.y;
                self.caret.x = self.caret_start.x;
                self.line_size = 0.0;
            },
            LineDir::Vertical => {
                self.caret.x += self.line_size + self.style.margin.x;
                self.caret.y = self.caret_start.y;
                self.line_size = 0.0;
            },
        }
    }
    
    /// Draws a separator and advances to the next line. See [`next_line`] for more info.
    ///
    /// [`next_line`]: struct.Ui.html#method.next_line
    pub fn line_separator(&mut self) {
        match self.line_dir {
            LineDir::Horizontal => {
                let width = self.style.separator_width;
                let color = self.style.separator_color;

                let a = Vec2 {
                    x: self.caret_start.x + width/2.0, 
                    y: self.caret.y + self.line_size,
                };
                let b = Vec2 {
                    x: self.caret.x - self.style.margin.x - width, 
                    y: a.y,
                };

                line(&mut self.draw_data, a, b, width, color);
            },
            LineDir::Vertical => {
                let a = Vec2::new(self.caret_start.x, self.caret.y + self.line_size + self.style.margin.y/2.0); 
                let b = Vec2::new(self.caret.x, self.caret.y + self.line_size + self.style.margin.y/2.0); 
                let width = self.style.separator_width;
                let color = self.style.separator_color;
                line(&mut self.draw_data, a, b, width, color);
            },
        }
        self.next_line();
    }

    /// Shows a new toggle button which toggles between showing `on_text` and `off_text` whenever
    /// the button is pressed. Note that this function needs to be called every frame if you want
    /// to see the button. Only the `on_text` is used to create the id for this button, so only it
    /// needs to be unique. Additionally, there is a separate id "pool" for toggle buttons, so you
    /// can have buttons and toggle buttons with the same nameThe same rules as for naming buttons 
    /// apply.
    ///
    /// Returns true if the button was toggled
    pub fn toggle_button(&mut self, on_text: &str, off_text: &str, state: &mut bool) -> bool {
        let (id, on_text) = id_and_text(on_text, CompType::ToggleButton, &self.salt);
        let text = if *state { on_text } else { off_text };
        let toggle = self.button_internal(text, id).2;
        if toggle {
            *state = !*state;
        }
        toggle
    }

    /// Shows a new button with the given text. Returns true if the button was pressed. Note that 
    /// this function needs to be called every frame if you want to see the button.
    ///
    /// Every button should have a unique display text. Buttons with the same name will behave
    /// like a singe button. If the text contains the character sequence "##", that sequence and
    /// any subsequent characters will not be shown. Using this, you can have multiple buttons
    /// show the same text.
    pub fn button(&mut self, text: &str) -> bool {
        let (id, text) = id_and_text(text, CompType::Button, &self.salt);
        self.button_internal(text, id).2
    }

    /// Internal version of the `button` method, which allows specifying a separate id for 
    /// the button. This allows a button to be used as the "host" for another component.
    ///
    /// Returns `(width, height, pressed)`.
    ///
    /// The text passed to this function will be displayed on the button directly, without checking
    /// for a "##" sequence.
    fn button_internal(&mut self, text: &str, id: Id) -> (f32, f32, bool) {
        let width = self.style.comp_width;
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
        self.draw_comp(pos, width, height, color, text, Alignment::Center);

        let pressed = self.held == Some(id) && hovered && self.mouse_state.released();
        (width, height, pressed)
    }

    /// Inserts the given string into the gui. If an alignment is given the label will have the
    /// default component size, and the text in it will be drawn based on that alignment. Returns
    /// true if the label is currently hovered.
    pub fn label(&mut self, text: &str, alignment: Option<Alignment>) {
        let (width, height, actual_alignment);

        match alignment {
            Some(alignment) => {
                actual_alignment = alignment;
                width = self.style.comp_width;
                height = self.default_height();
            },
            None => {
                actual_alignment = Alignment::Left;
                width = self.font.font().width(text, FONT_SIZE);
                height = self.default_height();
            },
        }

        let size = Vec2::new(width, height);
        let pos = self.caret;

        text_in_quad(&mut self.font, pos, size, self.style.padding, 
                     text, actual_alignment, self.style.text_color);
        self.advance_caret(width, height);
    }

    /// Inserts the given string into the gui. If an alignment is given the label will have the
    /// default component size, and the text in it will be drawn based on that alignment. This
    /// label differs from the normal label in that it can be hovered, and will change color if 
    /// hovered. Returns true if the label is currently hovered.
    pub fn label_hover(&mut self, text: &str, alignment: Option<Alignment>) -> bool {
        let (width, height, actual_alignment);

        match alignment {
            Some(alignment) => {
                actual_alignment = alignment;
                width = self.style.comp_width;
                height = self.default_height();
            },
            None => {
                actual_alignment = Alignment::Left;
                width = self.font.font().width(text, FONT_SIZE);
                height = self.default_height();
            },
        }

        let size = Vec2::new(width, height);
        let pos = self.caret;

        let hovered = self.mouse_pos.x > pos.x && self.mouse_pos.y > pos.y && 
            self.mouse_pos.x < pos.x + width && self.mouse_pos.y < pos.y + height;

        let color = if hovered { self.style.text_color_hovered } else { self.style.text_color };

        text_in_quad(&mut self.font, pos, size, self.style.padding, 
                     text, actual_alignment, color);
        self.advance_caret(width, height);

        hovered
    }

    /// Creates a new slider that allows selecting values from the given range. Returns a value
    /// from within the range.
    ///
    /// Every slider should have a unique display text. Buttons with the same name will behave
    /// like a singe slider. If the text contains the character sequence "##", that sequence and
    /// any subsequent characters will not be shown. Using this, you can have multiple sliders
    /// show the same text.
    pub fn slider(&mut self, text: &str, range: Range<f32>) -> f32 {
        let id = Id::from_str(text, CompType::Slider, &self.salt);
        let mut value = *self.slider_map.entry(id).or_insert((range.start + range.end) / 2.0);
        self.slider_ptr(text, range, &mut value);
        self.slider_map.insert(id, value);
        value
    }

    /// Creates a new slider that allows selecting values from the given range. The initial value
    /// is taken from `vaule`, and the selected value will be stored in that variable as well.
    /// Returns true if the value was changed.
    ///
    /// Every slider should have a unique display text. Buttons with the same name will behave
    /// like a singe slider. If the text contains the character sequence "##", that sequence and
    /// any subsequent characters will not be shown. Using this, you can have multiple sliders
    /// show the same text.
    pub fn slider_ptr(&mut self, text: &str, range: Range<f32>, value: &mut f32) -> bool {
        let (id, text) = id_and_text(text, CompType::Slider, &self.salt);

        let width = self.style.comp_width;
        let height = self.default_height();
        let pos = self.caret;
        self.advance_caret(width, height);

        let hovered = self.mouse_pos.x > pos.x && self.mouse_pos.y > pos.y && 
                      self.mouse_pos.x < pos.x + width && self.mouse_pos.y < pos.y + height;
        if hovered && self.mouse_state.pressed() {
            self.held = Some(id);
        } 

        let slider_size = {
            let size = height - self.style.padding.y;
            Vec2::new(size, size)
        };
        let slider_pos = {
            let norm_value = (*value - range.start) / (range.end - range.start);
            let slide_distance = width - self.style.padding.x - slider_size.x;
            pos + Vec2::new(self.style.padding.x/2.0 + norm_value*slide_distance, self.style.padding.y/2.0)
        };

        self.internal_fmt_string.clear();
        write!(self.internal_fmt_string, "{}: {:.*}", text, 2, value).unwrap();

        let changed = if self.held == Some(id) {
            let new_value = {
                let new_value = (self.mouse_pos.x - pos.x - self.style.padding.x/2.0 - slider_size.x/2.0) /
                                (width - self.style.padding.x - slider_size.x);
                range.start + new_value*(range.end - range.start)
            };

            if new_value != *value {
                *value = new_value;
                true
            } else {
                false
            }
        } else { false };

        if *value < range.start { *value = range.start; }
        if *value > range.end   { *value = range.end; }

        // Main bar
        let color = if hovered && self.held != Some(id) { self.style.hover_color } else { self.style.base_color };
        let text = &self.internal_fmt_string.clone();
        self.draw_comp(pos, width, height, color, text, Alignment::Center);
        // Slidy thing
        let color = if self.held == Some(id) { self.style.top_hold_color } else { self.style.top_color };
        quad(&mut self.draw_data, slider_pos, slider_size, color);

        changed
    }

    /// Creates a new textbox. The title will not be displayed, but should be a unique identifier
    /// for this textbox.
    pub fn textbox(&mut self, title: &str) -> &str {
        let id = Id::from_str(title, CompType::Textbox, &self.salt);
        let pos = self.caret;

        let (width, height, pressed) = self.button_internal("", id);
        if pressed {
            if self.focused == Some(id) {
            } else {
                self.focused = Some(id);
                self.caret_blink_time = 0.0;

                let TextboxInfo { ref mut text, ref mut caret } = *self.textbox_map.entry(id).or_insert(Default::default());
                *caret = text.len();
            }

            // Place caret at correct location
            let click_pos = self.mouse_pos.x - pos.x - self.style.padding.x/2.0;
            let TextboxInfo { ref mut text, ref mut caret } = *self.textbox_map.entry(id).or_insert(Default::default());
            let (visible_range, _) = self.font.font().visible_area(&text, FONT_SIZE, width - self.style.padding.x, *caret);
            if let Some(clicked) = self.font.font().hovered_char(&text[visible_range.clone()], FONT_SIZE, click_pos) {
                *caret = visible_range.start + clicked;
            } else {
                *caret = text.len();
            }
        }

        // Editing
        if self.focused == Some(id) {
            let TextboxInfo { ref mut text, ref mut caret } = *self.textbox_map.entry(id).or_insert(Default::default());

            if self.move_left && *caret > 0 {
                *caret -= 1;
                // Align to char boundary
                while !text.is_char_boundary(*caret) && *caret > 0 { *caret -= 1; }
                self.caret_blink_time = 0.0;
            }
            if self.move_right {
                *caret += 1;
                if *caret > text.len() {
                    *caret = text.len();
                } else {
                    // Align to char boundary
                    while !text.is_char_boundary(*caret) && *caret < text.len() { *caret += 1; }
                }
                self.caret_blink_time = 0.0;
            }

            text.reserve(self.typed.len());
            for c in self.typed.chars() {
                match c {
                    // Backspace
                    '\x08' => {
                        if *caret == text.len() {
                            if let Some(removed) = text.pop() {
                                *caret -= removed.len_utf8();
                            }
                        } else if *caret > 0 {
                            let mut remove_index = *caret - 1;
                            while !text.is_char_boundary(remove_index) && remove_index > 0 { remove_index -= 1 }
                            let removed = text.remove(remove_index);
                            *caret -= removed.len_utf8();
                        }
                    }, 
                    // Delete
                    '\x7f' => {
                        if *caret < text.len() {
                            text.remove(*caret);
                        }
                    },
                    // Ignore all other control characters
                    e if e <= '\x1f' => {}, 
                    _ => {
                        text.insert(*caret, c);
                        *caret += c.len_utf8();
                    },
                }

                self.caret_blink_time = 0.0;
            }
        }

        // Drawing
        if let Some(&TextboxInfo { ref text, ref caret }) = self.textbox_map.get(&id) {
            let caret = if self.focused == Some(id) { *caret } else { text.len() };

            let (visible_range, draw_caret_pos) =
                self.font.font().visible_area(&text, FONT_SIZE,
                                              width - self.style.padding.x,
                                              caret);
            let slice = &text[visible_range];

            // Draw text
            let text_pos = {
                let text_start = self.style.padding.y/2.0 - self.font.font().descent(FONT_SIZE);
                pos + Vec2::new(self.style.padding.x/2.0, height - text_start)
            };
            self.font.cache(slice, FONT_SIZE, text_pos, self.style.text_color);

            // Draw caret
            if self.focused == Some(id) && self.caret_blink_time % (2.0*CARET_BLINK_RATE) < CARET_BLINK_RATE {
                quad(&mut self.draw_data,
                     pos + Vec2::new(draw_caret_pos + self.style.padding.x/2.0 - self.style.caret_width/2.0, self.style.padding.y/4.0),
                     Vec2::new(self.style.caret_width/2.0, height - self.style.padding.y/2.0),
                     self.style.caret_color);
            }

            &text
        } else {
            ""
        }
    }

    fn draw_comp(&mut self, pos: Vec2<f32>, width: f32, height: f32, color: Color, text: &str, alignment: Alignment) {
        let size = Vec2::new(width, height);
        quad(&mut self.draw_data, pos, size, color);
        text_in_quad(&mut self.font, pos, size, self.style.padding, text, alignment, self.style.text_color);
    } 

    fn advance_caret(&mut self, comp_width: f32, comp_height: f32) {
        match self.line_dir {
            LineDir::Horizontal => {
                self.caret.x += comp_width + self.style.margin.x;
                self.line_size = f32::max(comp_height, self.line_size);
            },
            LineDir::Vertical => {
                self.caret.y += comp_height + self.style.margin.y;
                self.line_size = f32::max(comp_width, self.line_size);
            },
        }
    }

    fn default_height(&self) -> f32 {
        self.font.font().line_height(FONT_SIZE) + self.style.padding.y
    }
} 

#[derive(Clone, Debug, Default)]
struct TextboxInfo {
    text: String,
    caret: usize,
}

#[derive(Clone, Debug)]
pub struct Style {
    pub base_color: Color,
    pub hover_color: Color,
    pub hold_color: Color,
    pub top_color: Color,
    pub top_hold_color: Color,
    pub caret_color: Color,
    pub separator_color: Color,
    pub text_color: Color,
    pub text_color_hovered: Color,

    pub padding: Vec2<f32>,
    pub margin: Vec2<f32>,
    pub caret_width: f32,
    pub separator_width: f32,
    pub comp_width: f32,
}
impl Default for Style {
    fn default() -> Style {
        Style {
            base_color:         Color::hex_int(0x4c4665),
            hover_color:        Color::hex_int(0x575074),
            hold_color:         Color::hex_int(0x413c56),
            top_color:          Color::hex_int(0x403147),
            top_hold_color:     Color::hex_int(0x2a2738),
            caret_color:        Color::hex_int(0xffffff),
            separator_color:    Color::hex_int(0xffffff),
            text_color:         Color::hex_int(0xffffff),
            text_color_hovered: Color::hex_int(0xccccdd),

            padding: Vec2::new(10.0, 6.0),
            caret_width: 2.0,
            separator_width: 2.0,
            margin: Vec2::new(5.0, 5.0),
            comp_width: 150.0,
        }
    }
}

pub enum LineDir {
    /// Components are layed out below each other
    Vertical,
    /// Components are layed out side by side
    Horizontal,
}

/// Defines how children is layed out within a parent
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alignment {
    /// The left edge of the child is at the left edge of the parent
    Left, 
    /// The center of the child is at the center of the parent
    Center, 
    /// The right edge of the child is at the right edge of the parent
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct Id(u64, CompType);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum CompType {
    Button,
    ToggleButton,
    Slider,
    Textbox,
}

impl Id {
    fn from_str(text: &str, ty: CompType, salt: &str) -> Id {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hasher, Hash};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        salt.hash(&mut hasher);
        let id = hasher.finish();

        Id(id, ty)
    }
}
fn id_and_text<'a, 'b>(text: &'a str, ty: CompType, salt: &'b str) -> (Id, &'a str) {
    let id = Id::from_str(text, ty, salt);
    let name = text.split("##").next().unwrap();
    (id, name)
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

fn line(buf: &mut Vec<Vert>, a: Vec2<f32>, b: Vec2<f32>, width: f32, color: Color) {
    let normal = (b - a).normalize().left() * (width / 2.0);
    buf.push(Vert { pos: a - normal, color: color });
    buf.push(Vert { pos: b - normal, color: color });
    buf.push(Vert { pos: b + normal, color: color });
    buf.push(Vert { pos: a - normal, color: color });
    buf.push(Vert { pos: b + normal, color: color });
    buf.push(Vert { pos: a + normal, color: color });
}

fn text_in_quad(font: &mut CachedFont, pos: Vec2<f32>, size: Vec2<f32>, padding: Vec2<f32>, text: &str, alignment: Alignment, color: Color) {
    let text_pos = match alignment {
        Alignment::Left => {
            let text_start = padding.y/2.0 - font.font().descent(FONT_SIZE);
            pos + Vec2::new(padding.x/2.0, size.y - text_start)
        },
        Alignment::Right => {
            let text_width = font.font().width(text, FONT_SIZE);
            let text_v_offset = padding.y/2.0 - font.font().descent(FONT_SIZE);
            pos + Vec2::new(size.x - padding.x/2.0 - text_width, size.y - text_v_offset)
        },
        Alignment::Center => {
            let text_width = font.font().width(text, FONT_SIZE);
            let text_v_offset = padding.y/2.0 - font.font().descent(FONT_SIZE);
            pos + Vec2::new(size.x/2.0 - text_width/2.0, size.y - text_v_offset)
        },
    };
    font.cache(text, FONT_SIZE, text_pos, color);
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
    fn gen_shader_input_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_decl(_name_prefix: &str) -> String { String::new() }
    fn gen_transform_feedback_outputs(_name_prefix: &str) -> Vec<String> { Vec::new() }
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


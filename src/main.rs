
#![allow(dead_code)]

extern crate gl;
extern crate glutin;

mod framebuffer;
mod color;
mod texture;
mod shader;
mod primitive_buffer;
mod vertex_buffer;
mod vertex_array;

use glutin::*;
use framebuffer::*;
use color::*;
use shader::*;
use primitive_buffer::*;
use vertex_array::*;
use vertex_buffer::*;

use gl::types::*;
use std::time::{Instant, Duration};

const VERTEX_SOURCE: &'static str = "
    #version 330 core\n

    layout(location = 0) in vec2 in_pos;
    layout(location = 1) in vec4 in_color;

    out vec4 vert_color;

    void main() {
        gl_Position = vec4(in_pos, 0.0, 1.0);
        vert_color = in_color;
    }
";
const FRAGMENT_SOURCE: &'static str = "
    #version 330 core\n

    in vec4 vert_color;
    out vec4 out_color;

    void main() {
        out_color = vert_color;
    }
";

struct TestVertex {
    position: (f32, f32),
    color: Color
}
impl TestVertex {
    fn new(x: f32, y: f32) -> TestVertex {
        TestVertex {
            position: (x, y),
            color: Color::hex("ff00aa")
        }
    }
}
impl Vertex for TestVertex {
    fn bytes_per_vertex() -> usize {
        (2 + 4) * std::mem::size_of::<f32>()
    }
    fn setup_attrib_pointers() {
        unsafe {
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0 as GLuint,
                2 as GLint, gl::FLOAT, false as GLboolean,
                (6*std::mem::size_of::<f32>()) as GLsizei,
                (0*std::mem::size_of::<f32>()) as *const GLvoid
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1 as GLuint,
                3 as GLint, gl::FLOAT, false as GLboolean,
                (6*std::mem::size_of::<f32>()) as GLsizei,
                (2*std::mem::size_of::<f32>()) as *const GLvoid
            );
        }
    }
}

fn main() {
    let clear_color = Color::hex("ff34aa");
    let clear_color = clear_color.with_lightness(4.0);

    let window = glutin::Window::new().unwrap();

    unsafe {
        window.make_current().unwrap();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }

    let mut mouse_pos: (i32, i32) = (0, 0);
    let mut window_size = window.get_inner_size_points().unwrap();

    let mut framebuffer =
        FramebufferProperties::new(window_size.0, window_size.1)
        .build().unwrap();

    let shader = Shader::new(VERTEX_SOURCE, None, Some(FRAGMENT_SOURCE)).unwrap();
    shader.bind();

    let mut vbo = PrimitiveBuffer::new(BufferTarget::ArrayBuffer, BufferUsage::StaticDraw, DataType::Float);
    let vao = VertexArray::new();
    vao.add_data_source(&vbo, 0, 2, 2, 0);

    let test_data = vec![
        TestVertex::new(0.0, 0.0),
        TestVertex::new(1.0, 0.0),
        TestVertex::new(0.0, 1.0),
    ];
    let mut vertex_buffer = VertexBuffer::from_data(PrimitiveMode::Triangles, &test_data);

    let mut line_buffer = VertexBuffer::new(PrimitiveMode::LineStrip, BufferUsage::StaticDraw);

    let mut delta: u64 = 16;
    let target_delta = Duration::from_millis(14);

    'main_loop:
    loop {
        let start_time = Instant::now();

        let screen_pos = (
            (mouse_pos.0 as f32 / window_size.0 as f32)*2.0 - 1.0,
            1.0 - (mouse_pos.1 as f32 / window_size.1 as f32)*2.0,
        );

        for event in window.poll_events() {
            match event {
                Event::Closed => break 'main_loop,
                Event::Resized(width, height) => {
                    window_size = (width, height);
                    framebuffer =
                        FramebufferProperties::new(width, height)
                        .build().unwrap();
                },
                Event::MouseMoved(x, y) => {
                    mouse_pos = (x, y);
                },
                Event::MouseInput(ElementState::Pressed, MouseButton::Left) => {
                    println!("Mouse pressed");
                    line_buffer.put_at_end(&[TestVertex::new(screen_pos.0, screen_pos.1)]);
                },
                e => println!("{:?}", e)
            }
        }

        let new_data = vec![
            0.0, 0.0,
            1.0, 0.0,
            screen_pos.0, screen_pos.1
        ];
        vbo.put_floats(new_data);

        vertex_buffer.put(0, &[TestVertex::new(screen_pos.0, screen_pos.1)]);

        framebuffer.bind();
        framebuffer::clear(&clear_color);
        vertex_buffer.draw();
        line_buffer.draw();
        vao.draw(PrimitiveMode::Triangles, 0..3);
        framebuffer.blit();

        window.swap_buffers().unwrap();

        // Ensure loop runs at aprox. target delta
        if start_time.elapsed() < target_delta {
            std::thread::sleep(target_delta - start_time.elapsed()); // This is not very precice :/
        }
        let delta_dur = start_time.elapsed();
        delta = delta_dur.as_secs()*1000 + (delta_dur.subsec_nanos() as u64)/1000000;
    }
}


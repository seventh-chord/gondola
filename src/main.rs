
#![allow(dead_code)]

extern crate gl;
extern crate glutin;

mod framebuffer;
mod color;
mod texture;
mod shader;
mod graphics_buffer;
mod vertex_array;

use glutin::*;
use framebuffer::*;
use color::*;
use shader::*;
use graphics_buffer::*;
use vertex_array::*;

const VERTEX_SOURCE: &'static str = "
    #version 330 core\n

    layout(location = 0) in vec2 position;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
    }
";
const FRAGMENT_SOURCE: &'static str = "
    #version 330 core\n

    out vec4 out_color;
    void main() {
        out_color = vec4(0.6, 0.6, 1.0, 1.0);
    }
";

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

    let data = vec![
        0.0, 0.0,
        1.0, 0.0,
        1.0, 1.0
    ];
    let mut vbo = GraphicsBuffer::from_floats(BufferTarget::ArrayBuffer, data);
    let vao = VertexArray::new();
    vao.add_data_source(&vbo, 0, 2, 2, 0);

    for event in window.wait_events() {
        let new_data = vec![
            0.0, 0.0,
            1.0, 0.0,
            (mouse_pos.0 as f32 / window_size.0 as f32)*2.0 - 1.0,
            1.0 - (mouse_pos.1 as f32 / window_size.1 as f32)*2.0,
        ];
        vbo.put_floats(new_data);

        framebuffer.bind();
        framebuffer::clear(&clear_color);

        vao.draw(PrimitiveMode::Triangles, 0..3);

        framebuffer.blit();

        window.swap_buffers().unwrap();

        match event {
            Event::Closed => break,
            Event::Resized(width, height) => {
                window_size = (width, height);
                framebuffer =
                    FramebufferProperties::new(width, height)
                    .build().unwrap();
            },
            Event::MouseMoved(x, y) => {
                mouse_pos = (x, y);
            },
            e => println!("{:?}", e)
        }
    }
}


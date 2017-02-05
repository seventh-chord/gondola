
#![allow(dead_code)]

extern crate gl;
extern crate glutin;

mod framebuffer;
mod color;
mod texture;
mod shader;

use framebuffer::*;
use color::*;
use shader::*;

const VERTEX_SOURCE: &'static str = "
    #version 330 core\n

    in vec2 position;
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

    let window_size = window.get_inner_size_points().unwrap();
    let framebuffer_properties = FramebufferProperties::new(window_size.0, window_size.1);
    let framebuffer = framebuffer_properties.build().unwrap();
    //TODO: Framebuffer resizing

    let shader = Shader::new(VERTEX_SOURCE, None, Some(FRAGMENT_SOURCE)).unwrap();
    shader.bind();

    for event in window.wait_events() {
        framebuffer.bind();
        framebuffer::clear(&clear_color);

        framebuffer.blit();

        window.swap_buffers().unwrap();

        match event {
            glutin::Event::Closed => break,
            e => println!("{:?}", e)
        }
    }
}


extern crate gl;
extern crate glutin;

mod framebuffer;
mod texture;

use framebuffer::*;

const CLEAR_COLOR: (f32, f32, f32, f32) = (1.0, 0.9, 0.9, 1.0);

fn main() {
    let window = glutin::Window::new().unwrap();

    unsafe {
        window.make_current().unwrap();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }

    let window_size = window.get_inner_size_points().unwrap();
    let framebuffer_properties = FramebufferProperties::new(window_size.0, window_size.1);
    let framebuffer = framebuffer_properties.build().unwrap();

    for event in window.wait_events() {
        framebuffer.bind();
        framebuffer::clear(CLEAR_COLOR);

        framebuffer.blit();

        window.swap_buffers().unwrap();

        match event {
            glutin::Event::Closed => break,
            e => println!("{:?}", e)
        }
    }
}

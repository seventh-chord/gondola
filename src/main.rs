
extern crate gl;
extern crate glutin;

mod framebuffer;
mod texture;
#[macro_use]
mod color;

use framebuffer::*;
use color::*;


fn main() {
    let clear_color = hex!("ff34aa");

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
        framebuffer::clear(&clear_color);

        framebuffer.blit();

        window.swap_buffers().unwrap();

        match event {
            glutin::Event::Closed => break,
            e => println!("{:?}", e)
        }
    }
}

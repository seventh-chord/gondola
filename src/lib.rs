
extern crate gl;
extern crate glutin;
extern crate png;
extern crate cable_math;
extern crate rusttype;

pub mod color;
pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod matrix_stack;
pub mod util;
pub mod framebuffer;
pub mod font;

/// Creates a new window
pub fn create_window() -> glutin::Window {
    let window = glutin::Window::new().unwrap();
    unsafe {
        window.make_current().unwrap();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
    }
    window
}



#![allow(dead_code)]

extern crate gl;
extern crate glutin;
extern crate image;
extern crate cable_math;
#[macro_use]
extern crate gondola_vertex_macro;

pub mod framebuffer;
pub mod color;
pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod vertex_array;
pub mod matrix_stack;
pub mod util;

use framebuffer::*;
use color::*;
use buffer::*;
use vertex_array::*;
use matrix_stack::*;
use texture::*;
use shader::*;
use util::graphics;
use util::loading;

use glutin::*;
use gl::types::*;
use std::time::{Instant, Duration};
use cable_math::Vec2;

#[derive(Vertex)]
struct TestVertex {
    position: (f32, f32),
    color: Color,
}
impl TestVertex {
    fn new(x: f32, y: f32) -> TestVertex {
        TestVertex {
            position: (x, y),
            color: Color::hex("ff00aa"),
        }
    }
}

#[derive(Vertex)]
struct TileVertex {
    position: Vec2<f32>,
}
impl TileVertex {
    fn new(x: f32, y: f32) -> TileVertex {
        TileVertex {
            position: Vec2::new(x, y),
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

    let mut mouse_pos: (u32, u32) = (0, 0);
    let mut window_size = window.get_inner_size_points().unwrap();
    graphics::viewport(0, 0, window_size.0, window_size.1);

    let mut framebuffer = FramebufferProperties::new(window_size.0, window_size.1) .build().unwrap();

    let mut shader = load_shader!("assets/basic.glsl", TestVertex).unwrap();

    let tile_shader = load_shader!("assets/tile.glsl", TileVertex).unwrap();
    tile_shader.bind();
    tile_shader.set_uniform("size", (200.0, 200.0));

    let mut texture = Texture::from_file("assets/tile.png").unwrap();

    let mut vbo = PrimitiveBuffer::new(BufferTarget::Array, BufferUsage::StaticDraw, DataType::Float);
    let vao = VertexArray::new();
    vao.add_data_source(&vbo, 0, 2, 2, 0);

    let test_data = vec![
        TestVertex::new(0.0, 0.0),
        TestVertex::new(100.0, 0.0),
        TestVertex::new(0.0, 100.0),
    ];
    let mut vertex_buffer = VertexBuffer::from_data(PrimitiveMode::Triangles, &test_data);
    let mut line_buffer = VertexBuffer::new(PrimitiveMode::LineStrip, BufferUsage::StaticDraw);

    let tile_data = vec![
        TileVertex::new(100.0, 100.0),
        TileVertex::new(300.0, 100.0),
        TileVertex::new(300.0, 300.0),
        TileVertex::new(100.0, 300.0),
    ];
    let tile_buffer = VertexBuffer::from_data(PrimitiveMode::Points, &tile_data);

    let mut matrix_stack = MatrixStack::new();

    let mut delta: u64 = 16;
    let target_delta = Duration::from_millis(14);

    let mut resource_refresher = loading::ResourceRefresher::new();

    'main_loop:
    loop {
        // Hotload assets when not in release mode
        #[cfg(debug_assertions)] {
            if resource_refresher.check("assets/basic.glsl") {
                shader = load_shader!("assets/basic.glsl", TestVertex).unwrap();
            }
            if resource_refresher.check("assets/tile.png") {
                texture.load_file("assets/tile.png").unwrap_or_else(|err| println!("Failed to reload texture: {}", err));
            }
        }

        let start_time = Instant::now();

        for event in window.poll_events() {
            match event {
                Event::Closed => break 'main_loop,
                Event::Resized(width, height) => {
                    window_size = (width, height);
                    framebuffer = FramebufferProperties::new(window_size.0, window_size.1).build().unwrap();
                    matrix_stack.ortho(0.0, window_size.0 as f32, 0.0, window_size.1 as f32, -1.0, 1.0);
                    graphics::viewport(0, 0, window_size.0, window_size.1);
                },
                Event::MouseMoved(x, y) => {
                    mouse_pos = (x as u32, window_size.1 - y as u32);
                },
                Event::MouseInput(ElementState::Pressed, MouseButton::Left) => {
                    println!("Mouse pressed");
                    line_buffer.put_at_end(&[TestVertex::new(mouse_pos.0 as f32, mouse_pos.1 as f32)]);
                },
                e => println!("{:?}", e)
            }
        }

        matrix_stack.ortho(0.0, window_size.0 as f32, window_size.1 as f32, 0.0, -1.0, 1.0);

        let new_data = vec![
            0.0, 0.0,
            100.0, 0.0,
            mouse_pos.0 as f32, mouse_pos.1 as f32
        ];
        vbo.put_floats(new_data);

        vertex_buffer.put(0, &[TestVertex::new(mouse_pos.0 as f32, mouse_pos.1 as f32)]);

        framebuffer.bind();
        framebuffer::clear(&clear_color);
        {
            shader.bind();
            shader.set_uniform("mvp", matrix_stack.mvp());

            vertex_buffer.draw();
            line_buffer.draw();
            vao.draw(PrimitiveMode::Triangles, 0..3);

            tile_shader.bind();
            tile_shader.set_uniform("mvp", matrix_stack.mvp());

            texture.bind(0);
            tile_buffer.draw();
        }
        framebuffer.blit();

        window.swap_buffers().unwrap();

        graphics::print_errors();

        // Ensure loop runs at aprox. target delta
        let elapsed = start_time.elapsed();
        if elapsed < target_delta {
            std::thread::sleep(target_delta - elapsed); // This is not very precice :/
        }
        let delta_dur = start_time.elapsed();
        delta = delta_dur.as_secs()*1000 + (delta_dur.subsec_nanos() as u64)/1000000;
    }
}


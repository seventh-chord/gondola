﻿
extern crate gondola;
extern crate cable_math;

use gondola::{Window, WindowCommon, CursorType, Timer, InputManager, Key};
use gondola::Color;
use gondola::draw_group::{self, StateCmd};
use gondola::graphics;
use gondola::framebuffer::FramebufferProperties;
use cable_math::{Vec2, Mat4};

type DrawGroup = draw_group::DrawGroup<(), ()>;

fn main() {
    let mut timer = Timer::new();
    let mut input = InputManager::new();

    let gl_request = Default::default();
    let mut window = Window::new("This is hopefully still a window", gl_request);

    let mut draw_group = DrawGroup::new();

    let screen_size = window.screen_region().size().as_u32();
    let mut framebuffer_props = FramebufferProperties::new(screen_size);
    let mut framebuffer = framebuffer_props.build().unwrap();

    graphics::set_blending(Some(graphics::BlendSettings::default()));
    graphics::set_depth_testing(false);

    window.show();

    loop {
        let (time, delta) = timer.tick();

        window.poll_events(&mut input);

        let screen_region = window.screen_region();

        // Resize logic
        if window.resized() {
            framebuffer_props.size = screen_region.size().as_u32();
            framebuffer = framebuffer_props.build().unwrap();
        }

        // Update logic
        draw_group.reset();

        let bg_color = Color::hex_int(0xc0ffd5);
        draw_group.push_state_cmd(StateCmd::Clear(bg_color));

        draw_group.aabb((10.0, 10.0).into(), (100.0, 100.0).into(), Color::hex_int(0xff0000));

        let pos = Vec2::new(200.0, 200.0) + Vec2::polar(100.0, time.as_secs_float());
        draw_group.circle(pos, 10.0, Color::hex_int(0x00ff00));

        if input.key(Key::A).pressed_repeat() {
            println!("{}", delta.as_secs_float()*1000.0);
        }

        if input.key(Key::B).pressed() {
            window.set_vsync(true);
        }
        if input.key(Key::C).pressed() {
            window.set_vsync(false);
        }

        if input.key(Key::Key1).pressed() {
            window.change_title("Yo dawg");
        }

        if input.key(Key::Space).down() {
            window.set_cursor(CursorType::Clickable);
        } else {
            window.set_cursor(CursorType::Normal);
        }

        if input.key(Key::Key2).pressed() {
            window.change_title("Ding dong diddelido");
        }

        // Rendering logic
        let ortho = Mat4::ortho(
            0.0, screen_region.width(),
            0.0, screen_region.height(),
            -1.0, 1.0
        );

        framebuffer.bind();
        draw_group.draw(ortho, screen_region.size());
        framebuffer.blit(Default::default());

        window.swap_buffers();
        graphics::print_errors();

        if window.close_requested() {
            return;
        }
    }
}
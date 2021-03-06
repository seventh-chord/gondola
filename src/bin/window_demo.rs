﻿
extern crate gondola;
extern crate cable_math;

use gondola::{Window, WindowCommon, CursorType, Timer, Time, Input, Key, GamepadButton};
use gondola::Color;
use gondola::draw_group::{self, StateCmd};
use gondola::graphics;
use gondola::framebuffer::FramebufferProperties;
use gondola::audio::{AudioSystem, wav};
use cable_math::{Vec2, Mat4};

type DrawGroup = draw_group::DrawGroup<(), (), ()>;

fn main() {
    let mut timer = Timer::new();
    let mut input = Input::new();

    let mut window = Window::new("This is hopefully still a window");
    window.set_vsync(true);

    let mut audio = AudioSystem::initialize(&window);
    let hit_buffer = match wav::load("hit.wav") {
        Ok(b) => b,
        Err(err) => panic!("Oh snap: {}", err),
    };

    let hit_buffer_handle = audio.add_buffer(hit_buffer);

    let mut draw_group = DrawGroup::new();

    let screen_size = window.screen_region().size().as_u32();
    let mut framebuffer_props = FramebufferProperties::new(screen_size);
    let mut framebuffer = framebuffer_props.build().unwrap();

    graphics::set_blending(Some(graphics::BlendSettings::default()));
    graphics::set_depth_testing(false);

    window.show();


    let mut p = Vec2::ZERO;

    loop {
        let (time, _last_frame_time) = timer.tick();
        let delta = Time(Time::NANOSECONDS_PER_SECOND / 60);

        window.poll_events(&mut input);

        let screen_region = window.screen_region();

        // Resize logic
        if window.resized() {
            framebuffer_props.size = screen_region.size().as_u32();
            framebuffer = framebuffer_props.build().unwrap();
        }

        // Update logic
        if input.gamepads[0].connected {
            let ref g = input.gamepads[0];

            use GamepadButton::*;
            if g.button(RightUp).pressed()    { p.y -= 20.0; }
            if g.button(RightDown).pressed()  { p.y += 20.0; }
            if g.button(RightRight).pressed() { p.x += 20.0; }
            if g.button(RightLeft).pressed()  { p.x -= 20.0; }

            let s = delta.to_secs_f32() * 50.0;
            p.x += g.left.x * s;
            p.y -= g.left.y * s;
        }

        draw_group.reset();

        let bg_color = Color::hex_int(0xc0ffd5);
        draw_group.push_state_cmd(StateCmd::Clear(bg_color));

        draw_group.aabb(p - Vec2::new(10.0, 10.0), p + Vec2::new(10.0, 10.0), 0xff0000.into());

        let pos = Vec2::new(200.0, 200.0) + Vec2::polar(100.0, time.to_secs_f32());
        draw_group.circle(pos, 10.0, Color::hex_int(0x00ff00));

        if input.key(Key::A).pressed_repeat() {
            println!("{}", delta.to_secs_f32()*1000.0);
        }

        if input.key(Key::Key1).pressed() {
            window.change_title("Yo dawg");
        }

        if input.key(Key::Space).down() {
            window.set_cursor(CursorType::Clickable);
        } else {
            window.set_cursor(CursorType::Normal);
        }

        if input.mouse_keys[0].pressed() {
            let tx = input.mouse_pos.x / window.screen_region().width();
            let ty = input.mouse_pos.y / window.screen_region().height();

            audio.play(hit_buffer_handle, [1.0 - tx, tx], 0.5 + ty);
        }

        if input.mouse_keys[1].pressed() {
            let tx = input.mouse_pos.x / window.screen_region().width();
            let ty = input.mouse_pos.y / window.screen_region().height();

            audio.play(hit_buffer_handle, [1.0 - tx, tx], 1.0 + ty*0.5);
            audio.play(hit_buffer_handle, [tx, 1.0 - tx], 1.0 - ty*0.5);
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

        audio.tick();
        audio.print_potential_error();

        window.swap_buffers();
        graphics::print_errors();

        if window.close_requested() {
            return;
        }
    }
}

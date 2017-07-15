
extern crate gondola;
extern crate cable_math;

use gondola::{Window, Timer, InputManager, Key};
use gondola::Color;
use gondola::texture::TextureFormat;
use gondola::draw_group::{self, StateCmd};
use gondola::graphics;
use gondola::framebuffer::{self, FramebufferProperties};
use cable_math::{Vec2, Mat4};

type DrawGroup = draw_group::DrawGroup<(), ()>;

fn main() {
    let mut timer = Timer::new();
    let mut input = InputManager::new();
    let mut window = gondola::new_window("This is hopefully still a window");

    let mut draw_group = DrawGroup::new();

    let mut framebuffer_props = FramebufferProperties {
        width: window.screen_region().width() as u32,
        height: window.screen_region().height() as u32,
        .. Default::default()
    };
    framebuffer_props.color_formats[0] = Some(TextureFormat::RGBA_8);

    let mut framebuffer = framebuffer_props.build().unwrap();

    graphics::set_blending(Some(graphics::BlendSettings::default()));
    graphics::set_depth_testing(false);

    loop {
        let (time, delta) = timer.tick();

        window.poll_events(&mut input);
        let screen_region = window.screen_region();

        // Resize logic
        if window.resized() {
            let width = screen_region.width() as u32;
            let height = screen_region.height() as u32;

            if width != framebuffer_props.width || height != framebuffer_props.height {
                framebuffer_props.width = width;
                framebuffer_props.height = height;
                framebuffer = framebuffer_props.build().unwrap();
            }
        }

        // Update logic
        draw_group.reset();

        let bg_color = Color::hex_int(0xc0ffd5);
        draw_group.push_state_cmd(StateCmd::Clear(bg_color));

        draw_group.aabb((10.0, 10.0).into(), (100.0, 100.0).into(), Color::hex_int(0xff0000));

        let pos = Vec2::new(200.0, 200.0) + Vec2::polar(100.0, time.as_secs_float());
        draw_group.circle(pos, 10.0, Color::hex_int(0x00ff00));

        if input.key(Key::A).pressed_repeat() {
            println!("Ahh");
        }

        if input.mouse_key(2).pressed() {
            println!("{}", delta.as_secs_float()*1000.0);
        }

        // Rendering logic
        let ortho = Mat4::ortho(
            0.0, screen_region.width(),
            0.0, screen_region.height(),
            -1.0, 1.0
        );

        framebuffer.bind();
        draw_group.draw(ortho, screen_region.size());
        framebuffer.blit(framebuffer::BLIT_COLOR);

        window.swap_buffers();

        if window.close_requested() {
            return;
        }
    }
}

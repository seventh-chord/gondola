
extern crate gondola;
extern crate cable_math;

use gondola::{Window, WindowCommon, CursorType, Timer, Time, InputManager, Key};
use gondola::Color;
use gondola::draw_group::{self, StateCmd};
use gondola::graphics;
use gondola::framebuffer::FramebufferProperties;
use gondola::audio::{AudioSystem, wav};
use cable_math::{Vec2, Mat4};

type DrawGroup = draw_group::DrawGroup<(), ()>;

fn main() {
    let mut timer = Timer::new();
    let mut input = InputManager::new();

    let mut window = Window::new("This is hopefully still a window");
    window.set_vsync(true);

    let mut audio = AudioSystem::initialize(&window);
    let hit_buffer = match wav::load("dudu.wav") {
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
        draw_group.reset();

        let bg_color = Color::hex_int(0xc0ffd5);
        draw_group.push_state_cmd(StateCmd::Clear(bg_color));

        draw_group.aabb((10.0, 10.0).into(), (100.0, 100.0).into(), Color::hex_int(0xff0000));

        let pos = Vec2::new(200.0, 200.0) + Vec2::polar(100.0, time.as_secs_float());
        draw_group.circle(pos, 10.0, Color::hex_int(0x00ff00));

        if input.key(Key::A).pressed_repeat() {
            println!("{}", delta.as_secs_float()*1000.0);
        }

        if input.key(Key::Key1).pressed() {
            window.change_title("Yo dawg");
        }

        if input.key(Key::Space).down() {
            window.set_cursor(CursorType::Clickable);
        } else {
            window.set_cursor(CursorType::Normal);
        }

        if input.mouse_key(0).pressed() {
            let t = input.mouse_pos().x / window.screen_region().width();
            audio.play(hit_buffer_handle, [1.0 - t, t]);
        }

        if input.mouse_key(1).pressed() {
            let t = input.mouse_pos().x / window.screen_region().width();
            audio.play(hit_buffer_handle, [1.0 - t, t]);
            audio.play(hit_buffer_handle, [t, 1.0 - t]);
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

        window.swap_buffers();
        graphics::print_errors();

        if window.close_requested() {
            return;
        }
    }
}
